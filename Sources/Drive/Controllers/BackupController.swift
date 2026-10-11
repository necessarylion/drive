import Foundation
import Observation

/// Runs backup and restore for the window, and keeps their state.
@Observable
final class BackupController {
    var errorMessage = ""
    var isErrorShown = false
    var isExtrasDialogShown = false
    var isPaused = UserDefaults.standard.bool(forKey: AppConstants.awsPausedKey) {
        didSet { UserDefaults.standard.set(isPaused, forKey: AppConstants.awsPausedKey) }
    }

    private(set) var isRunning = false
    /// The saved access key ID, mostly hidden. Nil when no keys are saved in Settings.
    private(set) var savedKeyHint: String?
    /// The result of the last "Test Connection", in plain words.
    private(set) var connectionResult = ""
    private(set) var isTesting = false
    private(set) var lastResult = ""
    private var pendingPlan: BackupPlan?
    /// True when files changed while a backup was running.
    private var needsAnotherRun = false
    private var task: Task<Void, Never>?

    private let service: BackupService
    private let drive: DriveController

    init(service: BackupService, drive: DriveController) {
        self.service = service
        self.drive = drive
        refreshKeys(AWSConfig.saved().provider)
    }

    var isConfigured: Bool { !AWSConfig.saved().bucket.isEmpty }
    var fraction: Double { service.fraction }
    var isUploading: Bool { isRunning && service.uploadsTotal > 0 }
    var extrasCount: Int { pendingPlan?.extraKeys.count ?? 0 }

    var summaryText: String {
        if !isConfigured { return "Backup is off" }
        if isRunning { return service.progress.isEmpty ? "Working…" : service.progress }
        if isPaused { return "Backup is paused" }
        return lastResult.isEmpty ? "Waiting for the first backup" : lastResult
    }

    /// "s3://bucket/folder/"
    var destination: String {
        let config = AWSConfig.saved()
        return config.bucket.isEmpty ? "Backup is off" : config.destination
    }

    /// Nil means: show no backup icon for this item.
    func state(of item: DriveItem) -> SyncState? {
        if !isConfigured || item.isFolder || drive.section == .trash { return nil }
        return service.state(of: item)
    }

    /// Uploads new and changed files without asking. It never removes anything from the backup.
    // ponytail: each run lists the whole backup folder. Fine for a personal drive; keep a local index if it gets slow.
    func autoBackUp() {
        if !isConfigured || isPaused { return }
        if isRunning {
            needsAnotherRun = true
            return
        }
        task = Task { await runAuto() }
    }

    func togglePause() {
        isPaused.toggle()
        if isPaused {
            task?.cancel()
        } else {
            autoBackUp()
        }
    }

    /// The manual backup. It also asks what to do with files that are only in the backup.
    func backUp() {
        task = Task { await planBackup() }
    }

    func backUpAndRemoveExtras() {
        task = Task { await runBackup(removeExtras: true) }
    }

    func backUpAndKeepExtras() {
        task = Task { await runBackup(removeExtras: false) }
    }

    func restore() {
        task = Task { await runRestore() }
    }

    // MARK: - Settings

    /// Makes the settings from the Settings window the ones the backup uses. Until this is called,
    /// what the user types there changes nothing.
    func saveSettings(_ config: AWSConfig) {
        config.save()
        // A plan that waits for an answer was made for the old place.
        pendingPlan = nil
        lastResult = ""
        autoBackUp()
    }

    // MARK: - Access keys

    // The functions below take the settings that are on screen in the Settings window. They can be
    // not saved yet, so the user can set up and test a provider before the backup starts to use it.

    /// Returns true when the keys were saved.
    func saveKeys(accessKey: String, secretKey: String, for provider: BackupProvider) -> Bool {
        do {
            try service.saveKeys(accessKey: accessKey, secretKey: secretKey, for: provider)
        } catch {
            connectionResult = error.localizedDescription
            return false
        }
        refreshKeys(provider)
        connectionResult = "Keys saved."
        autoBackUp()
        return true
    }

    func removeKeys(_ provider: BackupProvider) {
        service.removeKeys(provider)
        savedKeyHint = nil
        connectionResult = "Keys removed."
    }

    /// The Google page where the user signs in. Nil when the settings are wrong; the reason is then shown.
    func googleSignInURL(_ config: AWSConfig) -> URL? {
        do {
            return try service.googleSignInURL(config)
        } catch {
            connectionResult = error.localizedDescription
            return nil
        }
    }

    /// Takes the address Google sent the user back to.
    func finishGoogleSignIn(_ callback: URL, _ config: AWSConfig) async {
        do {
            try await service.finishGoogleSignIn(callback, config)
        } catch {
            connectionResult = error.localizedDescription
            return
        }
        refreshKeys(config.provider)
        connectionResult = "Signed in."
        autoBackUp()
    }

    /// Each provider has its own keys, so Settings calls this when the provider changes.
    func refreshKeys(_ provider: BackupProvider) {
        savedKeyHint = service.savedKeyHint(provider)
        connectionResult = ""
    }

    func testConnection(_ config: AWSConfig) {
        Task { await runTest(config) }
    }

    private func runTest(_ config: AWSConfig) async {
        isTesting = true
        do {
            let count = try await service.testConnection(config)
            connectionResult = "It works. \(count) file(s) are in the backup."
        } catch {
            connectionResult = error.localizedDescription
        }
        isTesting = false
    }

    private func runAuto() async {
        isRunning = true
        do {
            let plan = try await service.plan(AWSConfig.saved())
            try await service.run(plan, removeExtras: false)
            lastResult = "All files are up to date"
        } catch {
            if !isCancellation(error) { lastResult = "Backup failed. Choose Back Up Now to see why." }
        }
        isRunning = false
        if needsAnotherRun {
            needsAnotherRun = false
            autoBackUp()
        }
    }

    private func planBackup() async {
        if isRunning { return }
        isRunning = true
        defer { isRunning = false }
        do {
            let plan = try await service.plan(AWSConfig.saved())
            pendingPlan = plan
            if plan.extraKeys.isEmpty {
                try await finish(plan, removeExtras: false)
            } else {
                // Removing from the backup cannot be undone, so the user decides.
                isExtrasDialogShown = true
            }
        } catch {
            fail(error)
        }
    }

    private func runBackup(removeExtras: Bool) async {
        guard let plan = pendingPlan, !isRunning else { return }
        isRunning = true
        defer { isRunning = false }
        do {
            try await finish(plan, removeExtras: removeExtras)
        } catch {
            fail(error)
        }
    }

    private func finish(_ plan: BackupPlan, removeExtras: Bool) async throws {
        try await service.run(plan, removeExtras: removeExtras)
        pendingPlan = nil
        lastResult = "All files are up to date"
    }

    private func runRestore() async {
        if isRunning { return }
        isRunning = true
        defer { isRunning = false }
        do {
            let count = try await service.restore(AWSConfig.saved())
            lastResult = "Restored \(count) file(s)"
        } catch {
            fail(error)
        }
        drive.refresh()
    }

    private func isCancellation(_ error: Error) -> Bool {
        error is CancellationError || (error as? URLError)?.code == .cancelled
    }

    private func fail(_ error: Error) {
        if isCancellation(error) { return }
        lastResult = "Last backup action failed"
        errorMessage = error.localizedDescription
        isErrorShown = true
    }
}
