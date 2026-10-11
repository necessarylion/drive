import AppKit
import Observation
import PDFKit
import UniformTypeIdentifiers

/// Holds what the window shows, and turns user actions into service calls.
@Observable
final class DriveController {
    private(set) var location = Location(section: .myDrive) {
        didSet {
            selection = []
            reload()
        }
    }
    var searchText = "" {
        didSet { reload() }
    }
    var sortField = SortField.name {
        didSet { reload() }
    }
    var isSortAscending = true {
        didSet { reload() }
    }
    /// Remembered between runs of the app.
    var viewMode = ViewMode(rawValue: UserDefaults.standard.string(forKey: AppConstants.viewModeKey) ?? "") ?? .list {
        didSet {
            UserDefaults.standard.set(viewMode.rawValue, forKey: AppConstants.viewModeKey)
            reload()
        }
    }
    var selection: Set<URL> = []
    var isInspectorShown = true
    /// Set to show a file in Quick Look.
    var previewURL: URL?
    /// Set to open a file in a viewer window. The main window watches it.
    var fileToView: URL?

    var prompt = NamePrompt.newFolder
    var promptText = ""
    var promptColor = FolderColor.blue
    var isPromptShown = false

    var errorMessage = ""
    var isErrorShown = false

    var pendingDeletion: Set<URL> = []
    var isDeletionDialogShown = false

    private(set) var items: [DriveItem] = []
    /// One list per open folder, from the root down. Only filled in column view.
    private(set) var columns: [[DriveItem]] = []
    private(set) var summaries: [URL: FolderSummary] = [:]
    private(set) var headerSubtitle = ""
    private(set) var pinned: [PinnedFolder] = []
    private(set) var slices: [StorageSlice] = []
    private(set) var recentFiles: [DriveItem] = []
    private(set) var usedBytes: Int64 = 0
    private(set) var isBusy = false
    /// Goes up after every change to the files. The backup watches it.
    private(set) var changeCount = 0

    private var backStack: [Location] = []
    private var forwardStack: [Location] = []
    private let service: DriveService

    init(service: DriveService) {
        self.service = service
        refresh()
    }

    var section: DriveSection { location.section }
    var path: [URL] { location.path }
    var currentFolder: URL { path.last ?? service.rootFolder }
    var canAddFiles: Bool { section == .myDrive }
    var canGoBack: Bool { !backStack.isEmpty }
    var canGoForward: Bool { !forwardStack.isEmpty }
    var isColumnMode: Bool { viewMode == .columns && section == .myDrive && searchText.isEmpty }

    var title: String {
        section == .myDrive ? (path.last?.lastPathComponent ?? section.title) : section.title
    }

    var statusText: String {
        let count = Plural.items(items.count)
        return selection.isEmpty ? count : "\(count), \(selection.count) selected"
    }

    var selectedItem: DriveItem? {
        items.first(where: { selection.contains($0.url) })
    }

    /// The row that is picked in the sidebar: a pinned folder when the user came in through one, else the list.
    var sidebarSelection: SidebarItem? {
        get {
            if let pinned = location.pinned { return .folder(pinned) }
            return .section(section)
        }
        set {
            switch newValue {
            case .section(let section): showSection(section)
            case .folder(let url): showPinned(url)
            case nil: break
            }
        }
    }

    /// How wide the inspector is now. The toolbar uses it to stop at the edge of the inspector.
    var inspectorWidth: CGFloat = 0

    /// The inspector is hidden in column view, because that view has its own preview.
    var isInspectorVisible: Bool {
        get { isInspectorShown && !isColumnMode }
        set { isInspectorShown = newValue }
    }

    /// The search field watches this. It takes the keyboard each time the number changes.
    private(set) var searchFocusCount = 0

    func focusSearch() {
        searchFocusCount += 1
    }

    func clearSearch() {
        searchText = ""
    }

    // MARK: - Reading

    /// Reads the current list again. Cheap enough to run on every key press in the search field.
    func reload() {
        let order = [sortField.comparator(ascending: isSortAscending)]
        items = service.items(in: section, folder: currentFolder, matching: searchText, sortedBy: order)
        selection.formIntersection(items.map(\.url))

        summaries = [:]
        for item in items where item.isFolder {
            summaries[item.url] = service.summary(of: item.url)
        }

        columns = []
        if isColumnMode {
            columns.append(service.children(of: service.rootFolder))
            for folder in path {
                columns.append(service.children(of: folder))
            }
        }

        var totalSize: Int64 = 0
        var latest = Date.distantPast
        for item in items {
            totalSize += summaries[item.url]?.size ?? item.size
            latest = max(latest, item.modified)
        }
        var parts = [Plural.items(items.count), totalSize.formatted(.byteCount(style: .file))]
        if !items.isEmpty { parts.append("Edited \(latest.formatted(.relative(presentation: .named)))") }
        headerSubtitle = parts.joined(separator: " · ")
    }

    /// Reads everything again, also the sidebar and the storage numbers.
    func refresh() {
        reload()
        pinned = service.pinnedFolders()
        slices = service.storageSlices()
        usedBytes = service.usedBytes()
        let newest = service.items(in: .recent, folder: service.rootFolder, matching: "", sortedBy: [])
        recentFiles = Array(newest.prefix(AppConstants.menuBarRecentLimit))
    }

    func color(of folder: URL) -> FolderColor {
        service.color(of: folder)
    }

    func areStarred(_ urls: Set<URL>) -> Bool {
        for item in items where urls.contains(item.url) && !item.isStarred {
            return false
        }
        return true
    }

    func arePinned(_ urls: Set<URL>) -> Bool {
        for url in urls where !service.isPinned(url) {
            return false
        }
        return true
    }

    func areFolders(_ urls: Set<URL>) -> Bool {
        for item in items where urls.contains(item.url) && !item.isFolder {
            return false
        }
        return true
    }

    /// "PDF document · 18.4 MB · 46 pages"
    func details(for item: DriveItem) -> String {
        if item.isFolder {
            let summary = summaries[item.url] ?? service.summary(of: item.url)
            return "Folder · \(summary.text)"
        }
        let type = UTType(filenameExtension: item.url.pathExtension)?.localizedDescription ?? "Document"
        var parts = [type.prefix(1).uppercased() + type.dropFirst(), item.sizeText]
        if item.kind == .pdf, let pages = PDFDocument(url: item.url)?.pageCount {
            parts.append("\(pages) pages")
        }
        return parts.joined(separator: " · ")
    }

    /// "My Drive / Clients / Northwind"
    func whereText(for item: DriveItem) -> String {
        if section == .trash { return DriveSection.trash.title }
        var names = [DriveSection.myDrive.title]
        for folder in service.folderChain(to: item.url.deletingLastPathComponent()) {
            names.append(folder.lastPathComponent)
        }
        return names.joined(separator: " / ")
    }

    func apps(for url: URL) -> [URL] {
        NSWorkspace.shared.urlsForApplications(toOpen: url)
    }

    func appName(_ app: URL) -> String {
        app.deletingPathExtension().lastPathComponent
    }

    func defaultAppName(for url: URL) -> String {
        guard let app = NSWorkspace.shared.urlForApplication(toOpen: url) else { return "Default App" }
        return appName(app)
    }

    // MARK: - Navigation

    func navigate(to target: Location) {
        var newLocation = target
        // Stay "inside" the pinned folder while the new place is still under it.
        if newLocation.pinned == nil, let pinned = location.pinned, newLocation.path.contains(pinned) {
            newLocation.pinned = pinned
        }
        if newLocation == location { return }
        backStack.append(location)
        forwardStack = []
        location = newLocation
    }

    func goBack() {
        guard let previous = backStack.popLast() else { return }
        forwardStack.append(location)
        location = previous
    }

    func goForward() {
        guard let next = forwardStack.popLast() else { return }
        backStack.append(location)
        location = next
    }

    func goToPathLevel(_ count: Int) {
        navigate(to: Location(section: .myDrive, path: Array(path.prefix(count))))
    }

    /// A click on a list in the sidebar. It always goes to the top of that list.
    func showSection(_ section: DriveSection) {
        searchText = ""
        navigate(to: Location(section: section))
    }

    /// A click on a pinned folder in the sidebar.
    func showPinned(_ url: URL) {
        searchText = ""
        navigate(to: Location(section: .myDrive, path: service.folderChain(to: url), pinned: url))
    }

    func showFolder(_ url: URL) {
        searchText = ""
        navigate(to: Location(section: .myDrive, path: service.folderChain(to: url)))
    }

    // MARK: - Selection

    func select(_ url: URL) {
        if NSEvent.modifierFlags.contains(.command) {
            selection.formSymmetricDifference([url])
        } else {
            selection = [url]
        }
    }

    /// A click in column view. A folder opens the next column. A file is picked.
    func selectInColumn(_ item: DriveItem, depth: Int) {
        let base = Array(path.prefix(depth))
        if item.isFolder {
            navigate(to: Location(section: .myDrive, path: base + [item.url]))
        } else {
            navigate(to: Location(section: .myDrive, path: base))
            selection = [item.url]
        }
    }

    func clearSelection() {
        selection = []
    }

    /// A right click on a selected item acts on the whole selection.
    func targets(for url: URL) -> Set<URL> {
        selection.contains(url) ? selection : [url]
    }

    func sort(by field: SortField) {
        if sortField == field {
            isSortAscending.toggle()
        } else {
            sortField = field
        }
    }

    func toggleInspector() {
        isInspectorShown.toggle()
    }

    func showInfo(_ urls: Set<URL>) {
        selection = urls
        isInspectorShown = true
    }

    // MARK: - Opening

    /// Opens a folder in place, or a file in a viewer window.
    func open(_ urls: Set<URL>) {
        if section == .trash { return }
        for item in items where urls.contains(item.url) {
            if item.isFolder {
                showFolder(item.url)
            } else {
                fileToView = item.url
            }
            return
        }
    }

    func openSelection() {
        open(selection)
    }

    func openInDefaultApp(_ url: URL) {
        NSWorkspace.shared.open(url)
    }

    func open(_ url: URL, with app: URL) {
        NSWorkspace.shared.open([url], withApplicationAt: app, configuration: NSWorkspace.OpenConfiguration())
    }

    func preview(_ urls: Set<URL>) {
        previewURL = items.first(where: { urls.contains($0.url) && !$0.isFolder })?.url
    }

    func previewSelection() {
        preview(selection)
    }

    func showInFinder(_ urls: Set<URL>) {
        NSWorkspace.shared.activateFileViewerSelecting(Array(urls))
    }

    func showDriveInFinder() {
        NSWorkspace.shared.open(service.rootFolder)
    }

    // MARK: - Adding

    func uploadFiles() {
        upload(folders: false)
    }

    func uploadFolder() {
        upload(folders: true)
    }

    func drop(_ urls: [URL], on folder: URL) -> Bool {
        if !canAddFiles || urls.isEmpty { return false }
        Task { await add(urls, to: folder) }
        return true
    }

    func askNewFolder() {
        prompt = .newFolder
        promptText = AppConstants.defaultFolderName
        promptColor = .blue
        isPromptShown = true
    }

    func askRename(_ urls: Set<URL>) {
        guard let url = urls.first else { return }
        prompt = .rename(url)
        promptText = url.lastPathComponent
        isPromptShown = true
    }

    func cancelPrompt() {
        isPromptShown = false
    }

    func confirmPrompt() {
        isPromptShown = false
        do {
            switch prompt {
            case .newFolder: try service.createFolder(named: promptText, in: currentFolder, color: promptColor)
            case .rename(let url): try service.rename(url, to: promptText)
            }
        } catch {
            fail(error)
        }
        didChange()
    }

    func newTextDocument() {
        newDocument(named: "Untitled.txt")
    }

    func newSpreadsheet() {
        newDocument(named: "Untitled.csv")
    }

    // MARK: - Changing

    func duplicate(_ urls: Set<URL>) {
        Task { await runDuplicate(urls) }
    }

    func duplicateSelection() {
        duplicate(selection)
    }

    func askMove(_ urls: Set<URL>) {
        guard let folder = chooseFolder(startingAt: currentFolder, button: "Move") else { return }
        if !service.isInDrive(folder) {
            fail(DriveError.outsideDrive)
            return
        }
        Task { await add(Array(urls), to: folder) }
    }

    func askDownloadCopy(_ urls: Set<URL>) {
        guard let folder = chooseFolder(startingAt: URL.downloadsDirectory, button: "Save Here") else { return }
        Task { await runExport(urls, to: folder) }
    }

    func toggleStar(_ urls: Set<URL>) {
        service.setStarred(urls, !areStarred(urls))
        didChange()
    }

    func togglePin(_ urls: Set<URL>) {
        service.setPinned(urls, !arePinned(urls))
        didChange()
    }

    func setColor(_ color: FolderColor, for urls: Set<URL>) {
        service.setColor(color, for: urls)
        didChange()
    }

    func trash(_ urls: Set<URL>) {
        do {
            try service.trash(urls)
        } catch {
            fail(error)
        }
        didChange()
    }

    func trashSelection() {
        if section == .trash {
            askDeleteForever(selection)
        } else {
            trash(selection)
        }
    }

    func restore(_ urls: Set<URL>) {
        do {
            try service.restore(urls)
        } catch {
            fail(error)
        }
        didChange()
    }

    func askDeleteForever(_ urls: Set<URL>) {
        if urls.isEmpty { return }
        pendingDeletion = urls
        isDeletionDialogShown = true
    }

    func askEmptyTrash() {
        askDeleteForever(Set(items.map(\.url)))
    }

    func confirmDeletion() {
        do {
            try service.deleteForever(pendingDeletion)
        } catch {
            fail(error)
        }
        pendingDeletion = []
        didChange()
    }

    // MARK: - Helpers

    private func didChange() {
        refresh()
        changeCount += 1
    }

    private func upload(folders: Bool) {
        let panel = NSOpenPanel()
        panel.canChooseFiles = !folders
        panel.canChooseDirectories = folders
        panel.allowsMultipleSelection = true
        panel.prompt = "Upload"
        if panel.runModal() == .OK {
            _ = drop(panel.urls, on: currentFolder)
        }
    }

    private func chooseFolder(startingAt folder: URL, button: String) -> URL? {
        let panel = NSOpenPanel()
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.canCreateDirectories = true
        panel.directoryURL = folder
        panel.prompt = button
        return panel.runModal() == .OK ? panel.url : nil
    }

    private func newDocument(named name: String) {
        do {
            let url = try service.createDocument(named: name, in: currentFolder)
            didChange()
            selection = [url]
        } catch {
            fail(error)
        }
    }

    private func add(_ urls: [URL], to folder: URL) async {
        isBusy = true
        do {
            try await service.add(urls, to: folder)
        } catch {
            fail(error)
        }
        isBusy = false
        didChange()
    }

    private func runDuplicate(_ urls: Set<URL>) async {
        isBusy = true
        do {
            try await service.duplicate(urls)
        } catch {
            fail(error)
        }
        isBusy = false
        didChange()
    }

    private func runExport(_ urls: Set<URL>, to folder: URL) async {
        isBusy = true
        do {
            try await service.export(urls, to: folder)
        } catch {
            fail(error)
        }
        isBusy = false
    }

    private func fail(_ error: Error) {
        errorMessage = error.localizedDescription
        isErrorShown = true
    }
}
