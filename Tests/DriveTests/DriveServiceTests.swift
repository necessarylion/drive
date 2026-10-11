import Foundation
import Testing

@testable import Drive

@MainActor
struct DriveServiceTests {
    private let config: AppConfig
    private let service: DriveService
    private let order = [KeyPathComparator(\DriveItem.name, comparator: .localizedStandard)]

    init() {
        let temp = FileManager.default.temporaryDirectory
        config = AppConfig(storageRoot: temp.appending(component: "DriveTests-\(UUID().uuidString)"))
        service = DriveService(
            config: config, files: FileRepository(), metadataStore: MetadataRepository(file: config.metadataFile))
    }

    private func names(_ section: DriveSection, in folder: URL? = nil) -> [String] {
        service.items(in: section, folder: folder ?? service.rootFolder, matching: "", sortedBy: order).map(\.name)
    }

    @Test func fullLifeOfAFile() async throws {
        defer { try? FileManager.default.removeItem(at: config.storageRoot) }
        let root = service.rootFolder

        // Import a file from outside, two times. The second one gets a new name.
        let outside = config.storageRoot.appending(component: "note.txt")
        try Data("hello".utf8).write(to: outside)
        try await service.add([outside, outside], to: root)
        #expect(names(.myDrive) == ["note 2.txt", "note.txt"])

        // Folders come first. A starred file keeps its star after a move and a rename.
        let folder = try service.createFolder(named: "Docs", in: root)
        let note = root.appending(component: "note.txt")
        service.setStarred([note], true)
        try await service.add([note], to: folder)
        try service.rename(folder.appending(component: "note.txt"), to: "todo.txt")
        #expect(names(.myDrive) == ["Docs", "note 2.txt"])
        #expect(names(.starred) == ["todo.txt"])

        // Bad names and bad moves are refused.
        #expect(throws: DriveError.invalidName) { try service.rename(folder, to: "a/b") }
        #expect(throws: DriveError.nameTaken("note 2.txt")) { try service.rename(folder, to: "note 2.txt") }
        let inner = try service.createFolder(named: "Inner", in: folder)
        await #expect(throws: DriveError.intoItself) { try await service.add([folder], to: inner) }

        // Trash, put back, then delete forever.
        try service.trash([folder])
        #expect(names(.myDrive) == ["note 2.txt"])
        #expect(names(.starred).isEmpty)
        let trashed = service.items(in: .trash, folder: root, matching: "", sortedBy: order)
        #expect(trashed.map(\.name) == ["Docs"])

        try service.restore([trashed[0].url])
        #expect(names(.myDrive, in: folder) == ["Inner", "todo.txt"])
        #expect(names(.starred) == ["todo.txt"])

        try service.trash([folder])
        // A path outside the trash must never be deleted by "delete forever".
        try service.deleteForever([root.appending(component: "note 2.txt")])
        #expect(names(.myDrive) == ["note 2.txt"])
        try service.deleteForever(
            Set(service.items(in: .trash, folder: root, matching: "", sortedBy: order).map(\.url)))
        #expect(names(.trash).isEmpty)
    }

    @Test func foldersKeepPinAndColorAfterRename() async throws {
        defer { try? FileManager.default.removeItem(at: config.storageRoot) }
        let root = service.rootFolder
        let folder = try service.createFolder(named: "Clients", in: root, color: .green)
        service.setPinned([folder], true)
        try service.rename(folder, to: "Customers")
        let renamed = root.appending(component: "Customers")

        #expect(service.pinnedFolders().map(\.url.lastPathComponent) == ["Customers"])
        #expect(service.color(of: renamed) == .green)
        #expect(service.isPinned(renamed))
        service.setPinned([renamed], false)
        #expect(service.pinnedFolders().isEmpty)
    }

    @Test func duplicatesExportsAndCountsFiles() async throws {
        defer { try? FileManager.default.removeItem(at: config.storageRoot) }
        let root = service.rootFolder
        let note = try service.createDocument(named: "Untitled.txt", in: root)
        try Data("hello".utf8).write(to: note)
        try await service.duplicate([note])
        try await service.duplicate([note])
        #expect(names(.myDrive) == ["Untitled copy 2.txt", "Untitled copy.txt", "Untitled.txt"])

        let summary = service.summary(of: root)
        #expect(summary.count == 3)
        #expect(summary.size == 15)
        #expect(service.storageSlices().map(\.group) == [.documents])

        let outside = config.storageRoot.appending(component: "Exported")
        try FileManager.default.createDirectory(at: outside, withIntermediateDirectories: true)
        try await service.export([note], to: outside)
        #expect(FileManager.default.fileExists(atPath: outside.appending(component: "Untitled.txt").path))
        #expect(service.isInDrive(note))
        #expect(!service.isInDrive(outside))
    }

    @Test func loadsMetadataSavedByAnOlderVersion() throws {
        let old = Data(#"{"starred":["/a/b.txt"],"trashOrigins":{}}"#.utf8)
        let metadata = try JSONDecoder().decode(DriveMetadata.self, from: old)
        #expect(metadata.starred == ["/a/b.txt"])
        #expect(metadata.pinned.isEmpty)
    }
}
