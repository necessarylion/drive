import Foundation

/// The rules of the drive: listing, adding, renaming, stars, pins and trash.
final class DriveService {
    private let config: AppConfig
    private let files: FileRepository
    private let metadataStore: MetadataRepository
    private var metadata: DriveMetadata
    private let filesRoot: URL
    private let trashRoot: URL

    var rootFolder: URL { filesRoot }

    init(config: AppConfig, files: FileRepository, metadataStore: MetadataRepository) {
        self.config = config
        self.files = files
        self.metadataStore = metadataStore
        self.filesRoot = files.preparedFolder(at: config.filesRoot)
        self.trashRoot = files.preparedFolder(at: config.trashRoot)
        self.metadata = metadataStore.load()
    }

    // MARK: - Reading

    func items(
        in section: DriveSection, folder: URL, matching query: String, sortedBy order: [KeyPathComparator<DriveItem>]
    ) -> [DriveItem] {
        var found: [DriveItem] = []
        switch section {
        case .myDrive:
            found = query.isEmpty ? files.contents(of: folder) : files.allItems(under: folder)
        case .recent:
            found = recentFiles()
        case .starred:
            for path in metadata.starred {
                if let item = files.item(at: URL(filePath: path)) { found.append(item) }
            }
        case .trash:
            for slot in files.contents(of: trashRoot) {
                found.append(contentsOf: files.contents(of: slot.url))
            }
        }

        var folders: [DriveItem] = []
        var others: [DriveItem] = []
        for var item in found where query.isEmpty || item.name.localizedCaseInsensitiveContains(query) {
            item.isStarred = metadata.starred.contains(item.url.path)
            if item.isFolder { folders.append(item) } else { others.append(item) }
        }
        // Recent keeps its newest-first order.
        if section == .recent { return others }
        return folders.sorted(using: order) + others.sorted(using: order)
    }

    /// What is inside one folder, sorted by name. Used by the column view.
    func children(of folder: URL) -> [DriveItem] {
        let byName = [KeyPathComparator(\DriveItem.name, comparator: .localizedStandard)]
        return items(in: .myDrive, folder: folder, matching: "", sortedBy: byName)
    }

    func summary(of folder: URL) -> FolderSummary {
        FolderSummary(count: files.contents(of: folder).count, size: files.totalSize(under: folder))
    }

    // ponytail: walks the whole tree on every call. Fine for thousands of files; cache it if it gets slow.
    func usedBytes() -> Int64 {
        files.totalSize(under: config.storageRoot)
    }

    /// How much space each group of file types uses. For the storage bar.
    func storageSlices() -> [StorageSlice] {
        var sizes: [StorageGroup: Int64] = [:]
        for file in files.allFiles(under: filesRoot) {
            sizes[file.kind.storageGroup, default: 0] += file.size
        }
        var slices: [StorageSlice] = []
        for group in StorageGroup.allCases {
            if let size = sizes[group], size > 0 { slices.append(StorageSlice(group: group, size: size)) }
        }
        return slices
    }

    func color(of folder: URL) -> FolderColor {
        metadata.folderColors[folder.path] ?? .blue
    }

    func isPinned(_ folder: URL) -> Bool {
        metadata.pinned.contains(folder.path)
    }

    func pinnedFolders() -> [PinnedFolder] {
        var folders: [PinnedFolder] = []
        for path in metadata.pinned {
            let url = URL(filePath: path, directoryHint: .notDirectory)
            if files.exists(at: url) { folders.append(PinnedFolder(url: url, color: color(of: url))) }
        }
        return folders
    }

    func isInDrive(_ url: URL) -> Bool {
        url.path == filesRoot.path || url.path.hasPrefix(filesRoot.path + "/")
    }

    /// The folders from the root down to the given folder. Used to jump to a folder.
    func folderChain(to folder: URL) -> [URL] {
        let rootPath = filesRoot.path
        var chain: [URL] = []
        var current = URL(filePath: folder.path, directoryHint: .notDirectory)
        while current.path.hasPrefix(rootPath + "/") {
            chain.insert(current, at: 0)
            current = URL(filePath: current.deletingLastPathComponent().path, directoryHint: .notDirectory)
        }
        return chain
    }

    // MARK: - Changing

    @discardableResult
    func createFolder(named name: String, in folder: URL, color: FolderColor = .blue) throws -> URL {
        let url = uniqueURL(named: try validated(name), in: folder)
        try files.createFolder(at: url)
        if color != .blue {
            metadata.folderColors[url.path] = color
            save()
        }
        return url
    }

    /// Makes a new empty file, like "Untitled.txt".
    @discardableResult
    func createDocument(named name: String, in folder: URL) throws -> URL {
        let url = uniqueURL(named: try validated(name), in: folder)
        try files.createFile(at: url)
        return url
    }

    /// Adds items to a folder. Items from outside the drive are copied. Items already in the drive are moved.
    func add(_ sources: [URL], to folder: URL) async throws {
        for source in sources {
            if source.path.hasPrefix(filesRoot.path + "/") {
                try move(source, to: folder)
            } else {
                try await files.copy(from: source, to: uniqueURL(named: source.lastPathComponent, in: folder))
            }
        }
    }

    /// "Report.pdf" gets a twin named "Report copy.pdf" in the same folder.
    func duplicate(_ urls: Set<URL>) async throws {
        for url in urls {
            let base = url.deletingPathExtension().lastPathComponent
            let ext = url.pathExtension
            let name = ext.isEmpty ? "\(base) copy" : "\(base) copy.\(ext)"
            try await files.copy(from: url, to: uniqueURL(named: name, in: url.deletingLastPathComponent()))
        }
    }

    /// Copies items to a folder outside the drive.
    func export(_ urls: Set<URL>, to folder: URL) async throws {
        for url in urls {
            try await files.copy(from: url, to: uniqueURL(named: url.lastPathComponent, in: folder))
        }
    }

    func rename(_ url: URL, to newName: String) throws {
        let name = try validated(newName)
        let oldName = url.lastPathComponent
        if name == oldName { return }
        let destination = url.deletingLastPathComponent().appending(component: name, directoryHint: .notDirectory)
        // A change of letter case only still "exists" on a Mac disk, so let that one pass.
        if files.exists(at: destination) && name.lowercased() != oldName.lowercased() {
            throw DriveError.nameTaken(name)
        }
        try files.move(from: url, to: destination)
        metadata.repath(from: url.path, to: destination.path)
        save()
    }

    func setStarred(_ urls: Set<URL>, _ starred: Bool) {
        for url in urls {
            if starred { metadata.starred.insert(url.path) } else { metadata.starred.remove(url.path) }
        }
        save()
    }

    func setPinned(_ urls: Set<URL>, _ pinned: Bool) {
        for url in urls {
            metadata.pinned.removeAll(where: { $0 == url.path })
            if pinned { metadata.pinned.append(url.path) }
        }
        save()
    }

    func setColor(_ color: FolderColor, for urls: Set<URL>) {
        for url in urls {
            metadata.folderColors[url.path] = color
        }
        save()
    }

    func trash(_ urls: Set<URL>) throws {
        defer { save() }
        // A parent and its child can both be picked. The child is gone once the parent moves.
        for url in urls where files.exists(at: url) {
            let slotID = UUID().uuidString
            let slot = trashRoot.appending(component: slotID, directoryHint: .notDirectory)
            try files.createFolder(at: slot)
            try files.move(
                from: url, to: slot.appending(component: url.lastPathComponent, directoryHint: .notDirectory))
            metadata.trashOrigins[slotID] = url.path
        }
    }

    func restore(_ urls: Set<URL>) throws {
        defer { save() }
        for url in urls {
            guard let slot = trashSlot(of: url) else { continue }
            var folder = filesRoot
            if let origin = metadata.trashOrigins[slot.lastPathComponent] {
                folder = URL(filePath: origin).deletingLastPathComponent()
            }
            try files.createFolder(at: folder)
            try files.move(from: url, to: uniqueURL(named: url.lastPathComponent, in: folder))
            try files.delete(at: slot)
            metadata.trashOrigins[slot.lastPathComponent] = nil
        }
    }

    func deleteForever(_ urls: Set<URL>) throws {
        defer { save() }
        for url in urls {
            guard let slot = trashSlot(of: url) else { continue }
            try files.delete(at: slot)
            metadata.trashOrigins[slot.lastPathComponent] = nil
        }
    }

    // MARK: - Helpers

    private func recentFiles() -> [DriveItem] {
        var recent: [DriveItem] = []
        for item in files.allItems(under: filesRoot) where !item.isFolder {
            recent.append(item)
        }
        recent.sort(using: KeyPathComparator(\DriveItem.modified, order: .reverse))
        return Array(recent.prefix(AppConstants.recentLimit))
    }

    private func move(_ source: URL, to folder: URL) throws {
        let sourcePath = source.path
        // Dropped where it already is: nothing to do.
        if source.deletingLastPathComponent().path == folder.path || folder.path == sourcePath { return }
        if folder.path.hasPrefix(sourcePath + "/") { throw DriveError.intoItself }
        let destination = uniqueURL(named: source.lastPathComponent, in: folder)
        try files.move(from: source, to: destination)
        metadata.repath(from: sourcePath, to: destination.path)
        save()
    }

    /// The trash slot folder of a trashed item. Nil when the item is not in the trash, so nothing else gets deleted.
    private func trashSlot(of url: URL) -> URL? {
        let slot = url.deletingLastPathComponent()
        return slot.deletingLastPathComponent().path == trashRoot.path ? slot : nil
    }

    private func validated(_ name: String) throws -> String {
        let trimmed = name.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty || trimmed.hasPrefix(".") || trimmed.contains("/") || trimmed.contains(":") {
            throw DriveError.invalidName
        }
        return trimmed
    }

    /// "Report.pdf" becomes "Report 2.pdf" when the name is taken.
    private func uniqueURL(named name: String, in folder: URL) -> URL {
        var candidate = folder.appending(component: name, directoryHint: .notDirectory)
        let base = candidate.deletingPathExtension().lastPathComponent
        let ext = candidate.pathExtension
        var counter = 2
        while files.exists(at: candidate) {
            let numbered = ext.isEmpty ? "\(base) \(counter)" : "\(base) \(counter).\(ext)"
            candidate = folder.appending(component: numbered, directoryHint: .notDirectory)
            counter += 1
        }
        return candidate
    }

    // ponytail: a failed save only loses stars, pins, colors and "put back" places, never files. Not shown to the user.
    private func save() {
        try? metadataStore.save(metadata)
    }
}
