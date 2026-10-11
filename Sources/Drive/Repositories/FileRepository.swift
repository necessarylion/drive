import Foundation

/// All access to files and folders on disk.
final class FileRepository {
    private let manager = FileManager.default
    private let keys: Set<URLResourceKey> = [
        .isDirectoryKey, .isPackageKey, .fileSizeKey, .contentModificationDateKey, .creationDateKey,
    ]

    /// Creates the folder and returns its real path.
    /// Listings return real paths ("/private/var", not "/var"), and path checks need both sides to match.
    func preparedFolder(at url: URL) -> URL {
        // If this fails, the first real action fails too and shows the error.
        try? createFolder(at: url)
        let realPath = try? url.resourceValues(forKeys: [.canonicalPathKey]).canonicalPath
        return URL(filePath: realPath ?? url.path, directoryHint: .notDirectory)
    }

    func item(at url: URL) -> DriveItem? {
        // A fresh URL without a trailing slash, so the same item always has the same URL.
        let clean = URL(filePath: url.path, directoryHint: .notDirectory)
        guard let values = try? clean.resourceValues(forKeys: keys) else { return nil }
        return DriveItem(
            url: clean,
            isFolder: values.isDirectory == true && values.isPackage != true,
            size: Int64(values.fileSize ?? 0),
            modified: values.contentModificationDate ?? .distantPast,
            created: values.creationDate ?? .distantPast)
    }

    func contents(of folder: URL) -> [DriveItem] {
        let urls = try? manager.contentsOfDirectory(
            at: folder, includingPropertiesForKeys: Array(keys), options: [.skipsHiddenFiles])
        var items: [DriveItem] = []
        for url in urls ?? [] {
            if let item = item(at: url) { items.append(item) }
        }
        return items
    }

    /// Every item under the folder, at any depth.
    func allItems(under folder: URL) -> [DriveItem] {
        let enumerator = manager.enumerator(
            at: folder, includingPropertiesForKeys: Array(keys), options: [.skipsHiddenFiles, .skipsPackageDescendants])
        var items: [DriveItem] = []
        while let url = enumerator?.nextObject() as? URL {
            if let item = item(at: url) { items.append(item) }
        }
        return items
    }

    /// Every real file under the folder, at any depth. Unlike `allItems`, it also looks inside packages.
    func allFiles(under folder: URL) -> [DriveItem] {
        let fileKeys = Array(keys) + [.isRegularFileKey]
        let enumerator = manager.enumerator(
            at: folder, includingPropertiesForKeys: fileKeys, options: [.skipsHiddenFiles])
        var found: [DriveItem] = []
        while let url = enumerator?.nextObject() as? URL {
            let isFile = (try? url.resourceValues(forKeys: [.isRegularFileKey]))?.isRegularFile == true
            if isFile, let item = item(at: url) { found.append(item) }
        }
        return found
    }

    func setModified(_ date: Date, at url: URL) throws {
        try manager.setAttributes([.modificationDate: date], ofItemAtPath: url.path)
    }

    func totalSize(under folder: URL) -> Int64 {
        let enumerator = manager.enumerator(at: folder, includingPropertiesForKeys: [.fileSizeKey])
        var total: Int64 = 0
        while let url = enumerator?.nextObject() as? URL {
            total += Int64((try? url.resourceValues(forKeys: [.fileSizeKey]))?.fileSize ?? 0)
        }
        return total
    }

    func exists(at url: URL) -> Bool {
        manager.fileExists(atPath: url.path)
    }

    func createFolder(at url: URL) throws {
        try manager.createDirectory(at: url, withIntermediateDirectories: true)
    }

    func createFile(at url: URL) throws {
        if !manager.createFile(atPath: url.path, contents: nil) {
            throw CocoaError(.fileWriteUnknown)
        }
    }

    func move(from source: URL, to destination: URL) throws {
        try manager.moveItem(at: source, to: destination)
    }

    func delete(at url: URL) throws {
        try manager.removeItem(at: url)
    }

    /// Copies off the main thread, so a big file does not freeze the window.
    @concurrent
    nonisolated func copy(from source: URL, to destination: URL) async throws {
        try FileManager.default.copyItem(at: source, to: destination)
    }
}
