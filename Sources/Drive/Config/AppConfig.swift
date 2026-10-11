import Foundation

/// Where the app keeps its data on disk.
nonisolated struct AppConfig: Sendable {
    let storageRoot: URL

    var filesRoot: URL { storageRoot.appending(component: "Files", directoryHint: .notDirectory) }
    var trashRoot: URL { storageRoot.appending(component: "Trash", directoryHint: .notDirectory) }
    var metadataFile: URL { storageRoot.appending(component: "metadata.json", directoryHint: .notDirectory) }
    var backupStateFile: URL { storageRoot.appending(component: "backup-state.json", directoryHint: .notDirectory) }

    static let defaultStorageRoot = URL.applicationSupportDirectory.appending(
        component: "Drive", directoryHint: .notDirectory)

    /// The folder picked in Settings, or the default one.
    /// Set DRIVE_STORAGE_ROOT to try the app with another folder, away from the real files.
    // ponytail: if the picked folder is missing (a disk that is not plugged in), the drive shows empty
    // and actions fail with an error. Add a check at start if this becomes a real case.
    static var live: AppConfig {
        if let custom = ProcessInfo.processInfo.environment["DRIVE_STORAGE_ROOT"] {
            return AppConfig(storageRoot: URL(filePath: custom, directoryHint: .notDirectory))
        }
        if let saved = UserDefaults.standard.string(forKey: AppConstants.storageRootKey), !saved.isEmpty {
            return AppConfig(storageRoot: URL(filePath: saved, directoryHint: .notDirectory))
        }
        return AppConfig(storageRoot: defaultStorageRoot)
    }
}
