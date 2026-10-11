import Foundation

/// Reads and writes the metadata JSON file.
final class MetadataRepository {
    private let file: URL

    init(file: URL) {
        self.file = file
    }

    func load() -> DriveMetadata {
        guard let data = try? Data(contentsOf: file) else { return DriveMetadata() }
        return (try? JSONDecoder().decode(DriveMetadata.self, from: data)) ?? DriveMetadata()
    }

    func save(_ metadata: DriveMetadata) throws {
        try JSONEncoder().encode(metadata).write(to: file, options: .atomic)
    }
}
