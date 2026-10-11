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
        savedKeyHint = service.savedKeyHint()
    }

    var isConfigured: Bool { !AWSConfig.saved().bucket.isEmpty }
    var fraction: Double { service.fraction }
    var isUploading: Bool { isRunning && service.uploadsTotal > 0 }
    var extrasCount: Int { pendingPlan?.extraKeys.count ?? 0 }

    var summaryText: String {
        if !isConfigured { return "AWS backup is off" }
        if isRunning { return service.progress.isEmpty ? "Working…" : service.progress }
        if isPaused { return "Backup is paused" }
        return lastResult.isEmpty ? "Waiting for the first backup" : lastResult
    }

    /// "s3://bucket/folder/"
    var destination: String {
        let config = AWSConfig.saved()
        return config.bucket.isEmpty ? "AWS backup is off" : "s3://\(config.bucket)/\(config.keyPrefix)"
    }

    /// Nil means: show no backup icon for this item.
    func state(of item: DriveItem) -> SyncState? {
        if !isConfigured || item.isFolder || drive.section == .trash { return nil }
        return service.state(of: item)
    }

    /// Uploads new and changed files without asking. It never removes anything from S3.
    // ponytail: each run lists the whole S3 folder. Fine for a personal drive; keep a local index if it gets slow.
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

    /// The manual backup. It also asks what to do with files that are only in S3.
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

    // MARK: - Access keys

    /// Returns true when the keys were saved.
    func saveKeys(accessKey: String, secretKey: String) -> Bool {
        do {
            try service.saveKeys(accessKey: accessKey, secretKey: secretKey)
        } catch {
            connectionResult = error.localizedDescription
            return false
        }
        savedKeyHint = service.savedKeyHint()
        connectionResult = "Keys saved."
        autoBackUp()
        return true
    }

    func removeKeys() {
        service.removeKeys()
        savedKeyHint = nil
        connectionResult = "Keys removed."
    }

    func testConnection() {
        Task { await runTest() }
    }

    private func runTest() async {
        isTesting = true
        do {
            let count = try await service.testConnection(AWSConfig.saved())
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
                // Removing from S3 cannot be undone, so the user decides.
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
        lastResult = "Last AWS action failed"
        errorMessage = error.localizedDescription
        isErrorShown = true
    }
}
