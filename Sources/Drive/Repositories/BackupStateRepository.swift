import Foundation

/// Reads and writes the backup state JSON file.
final class BackupStateRepository {
    private let file: URL

    init(file: URL) {
        self.file = file
    }

    func load() -> BackupState {
        guard let data = try? Data(contentsOf: file) else { return BackupState() }
        return (try? JSONDecoder().decode(BackupState.self, from: data)) ?? BackupState()
    }

    func save(_ state: BackupState) throws {
        try JSONEncoder().encode(state).write(to: file, options: .atomic)
    }
}
