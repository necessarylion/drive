import SwiftUI

@main
struct DriveApp: App {
    @State private var container = AppContainer()

    var body: some Scene {
        Window("Drive", id: AppConstants.mainWindowID) {
            ContentView()
                .environment(container.drive)
                .environment(container.backup)
                .frame(minWidth: 900, minHeight: 520)
        }
        .defaultSize(width: 1280, height: 780)
        .commands {
            DriveCommands(drive: container.drive, backup: container.backup, searchFocuser: container.searchFocuser)
        }

        WindowGroup("Viewer", id: AppConstants.viewerWindowID, for: URL.self) { $url in
            if let url {
                ViewerView(url: url)
                    .environment(container.drive)
            }
        }
        .defaultSize(width: 1000, height: 780)
        // A viewer window from the last run could point at a file that is gone.
        .restorationBehavior(.disabled)

        MenuBarExtra("Drive", systemImage: "icloud.and.arrow.up") {
            MenuBarView()
                .environment(container.drive)
                .environment(container.backup)
        }
        .menuBarExtraStyle(.window)

        Settings {
            SettingsView()
                .environment(container.backup)
        }
    }
}

/// The one place where the parts are wired together.
final class AppContainer {
    let drive: DriveController
    let backup: BackupController
    let searchFocuser = SearchFieldFocuser()

    init() {
        AWSConfig.migrate()
        let config = AppConfig.live
        let files = FileRepository()
        let signer = AWSSigner()
        let googleAuth = GoogleAuthRepository(signer: signer)
        let driveService = DriveService(
            config: config, files: files, metadataStore: MetadataRepository(file: config.metadataFile))
        let backupService = BackupService(
            root: driveService.rootFolder, files: files, s3: S3Repository(signer: signer),
            azure: AzureRepository(signer: signer), google: GoogleDriveRepository(auth: googleAuth),
            googleAuth: googleAuth, keychain: KeychainRepository(service: AppConstants.awsKeychainService),
            stateStore: BackupStateRepository(file: config.backupStateFile))
        drive = DriveController(service: driveService)
        backup = BackupController(service: backupService, drive: drive)
    }
}

struct DriveCommands: Commands {
    let drive: DriveController
    let backup: BackupController
    let searchFocuser: SearchFieldFocuser

    var body: some Commands {
        CommandGroup(replacing: .newItem) {
            Button("New Folder", action: drive.askNewFolder)
                .keyboardShortcut("n", modifiers: [.shift, .command])
                .disabled(!drive.canAddFiles)
            Button("Upload Files…", action: drive.uploadFiles)
                .keyboardShortcut("u")
                .disabled(!drive.canAddFiles)
            Button("Upload Folder…", action: drive.uploadFolder)
                .disabled(!drive.canAddFiles)
            Divider()
            Button("Open", action: drive.openSelection)
                .keyboardShortcut("o")
                .disabled(drive.selection.isEmpty)
            Button("Quick Look", action: drive.previewSelection)
                .keyboardShortcut("y")
                .disabled(drive.selection.isEmpty)
            Button("Duplicate", action: drive.duplicateSelection)
                .keyboardShortcut("d")
                .disabled(drive.selection.isEmpty || !drive.canAddFiles)
            Button("Show Inspector", action: drive.toggleInspector)
                .keyboardShortcut("i")
            Button("Move to Trash", action: drive.trashSelection)
                .keyboardShortcut(.delete)
                .disabled(drive.selection.isEmpty)
        }
        CommandGroup(replacing: .textEditing) {
            Button("Find", action: searchFocuser.focus)
                .keyboardShortcut("f")
        }
        CommandMenu("Go") {
            Button("Back", action: drive.goBack)
                .keyboardShortcut("[")
                .disabled(!drive.canGoBack)
            Button("Forward", action: drive.goForward)
                .keyboardShortcut("]")
                .disabled(!drive.canGoForward)
        }
        CommandMenu("Backup") {
            Button("Back Up Now", action: backup.backUp)
                .disabled(backup.isRunning)
            Button("Restore from Backup", action: backup.restore)
                .disabled(backup.isRunning)
            Divider()
            Button(backup.isPaused ? "Resume Backup" : "Pause Backup", action: backup.togglePause)
                .disabled(!backup.isConfigured)
        }
    }
}
