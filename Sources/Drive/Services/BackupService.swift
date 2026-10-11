import Foundation
import Observation

/// Copies the drive to the backup provider, and brings it back.
@Observable
final class BackupService {
    /// A short line about the current step, for the window to show.
    private(set) var progress = ""
    private(set) var uploadsDone = 0
    private(set) var uploadsTotal = 0
    private var uploadingPath = ""
    /// File path -> the change date that is known to be in the backup.
    private var syncedDates: [String: Date] = [:]

    private let root: URL
    private let files: FileRepository
    private let s3: any BackupRepository
    private let azure: any BackupRepository
    private let google: any BackupRepository
    private let googleAuth: GoogleAuthRepository
    private let keychain: KeychainRepository

    init(
        root: URL, files: FileRepository, s3: any BackupRepository, azure: any BackupRepository,
        google: any BackupRepository, googleAuth: GoogleAuthRepository, keychain: KeychainRepository
    ) {
        self.root = root
        self.files = files
        self.s3 = s3
        self.azure = azure
        self.google = google
        self.googleAuth = googleAuth
        self.keychain = keychain
    }

    var fraction: Double {
        uploadsTotal == 0 ? 0 : Double(uploadsDone) / Double(uploadsTotal)
    }

    // ponytail: known only after the first check with the backup in this run of the app. Before that, files show as pending.
    func state(of item: DriveItem) -> SyncState {
        if item.url.path == uploadingPath { return .uploading }
        if let date = syncedDates[item.url.path], item.modified <= date { return .synced }
        return .pending
    }

    /// Compares this Mac with the backup. Changes nothing.
    func plan(_ config: AWSConfig) async throws -> BackupPlan {
        defer { progress = "" }
        let connection = try connect(config)
        progress = "Checking the backup…"
        var remote: [String: S3Object] = [:]
        for object in try await repository(config).list(connection) {
            remote[object.key] = object
        }

        var plan = BackupPlan(connection: connection)
        for file in files.allFiles(under: root) {
            let key = config.keyPrefix + file.url.path.dropFirst(root.path.count + 1)
            let existing = remote.removeValue(forKey: key)
            // Same rule as "aws s3 sync": upload when the size differs or the local file is newer.
            if existing?.size != file.size || file.modified > existing?.lastModified ?? .distantPast {
                plan.uploads.append(BackupUpload(file: file.url, key: key, modified: file.modified))
            } else {
                syncedDates[file.url.path] = file.modified
            }
        }
        // Keys that end with "/" are folder markers made by the provider's website. Leave them.
        for key in remote.keys.sorted() where !key.hasSuffix("/") {
            plan.extraKeys.append(key)
        }
        return plan
    }

    func run(_ plan: BackupPlan, removeExtras: Bool) async throws {
        defer {
            progress = ""
            uploadingPath = ""
            uploadsDone = 0
            uploadsTotal = 0
        }
        let remote = repository(plan.connection.config)
        uploadsTotal = plan.uploads.count
        for upload in plan.uploads {
            progress = "Uploading \(uploadsDone + 1) of \(uploadsTotal)"
            uploadingPath = upload.file.path
            try await remote.upload(upload.file, key: upload.key, plan.connection)
            syncedDates[upload.file.path] = upload.modified
            uploadsDone += 1
        }
        uploadingPath = ""
        if !removeExtras { return }
        for (index, key) in plan.extraKeys.enumerated() {
            progress = "Removing \(index + 1) of \(plan.extraKeys.count)"
            try await remote.delete(key: key, plan.connection)
        }
    }

    /// Downloads the files that are in the backup but not on this Mac. Never replaces a local file.
    /// Returns how many files it downloaded.
    func restore(_ config: AWSConfig) async throws -> Int {
        defer { progress = "" }
        let connection = try connect(config)
        progress = "Checking the backup…"
        let remote = repository(config)
        var count = 0
        for object in try await remote.list(connection) {
            let relative = String(object.key.dropFirst(config.keyPrefix.count))
            // A key is outside data. Refuse "..", so nothing can be written outside the drive.
            let parts = relative.split(separator: "/", omittingEmptySubsequences: false)
            if parts.contains("") || parts.contains(".") || parts.contains("..") { continue }
            let destination = root.appending(path: relative, directoryHint: .notDirectory)
            if files.exists(at: destination) { continue }

            count += 1
            progress = "Downloading file \(count)"
            let downloaded = try await remote.download(key: object.key, connection)
            try files.createFolder(at: destination.deletingLastPathComponent())
            try files.move(from: downloaded, to: destination)
            // Keep the backup date, so the next backup does not upload the file again.
            try files.setModified(object.lastModified, at: destination)
            syncedDates[destination.path] = object.lastModified
        }
        return count
    }

    // MARK: - Access keys

    /// The saved access key ID with most of it hidden, like "AKIA…3XYZ". Nil when no keys are saved.
    func savedKeyHint(_ provider: BackupProvider) -> String? {
        guard let key = keychain.load(provider)?.accessKey else { return nil }
        if provider == .googleDrive { return "Signed in" }
        // An Azure SAS token has no key ID to show.
        return key.isEmpty ? "SAS token saved" : "\(key.prefix(4))…\(key.suffix(4))"
    }

    /// For Azure, `secretKey` is the SAS token and `accessKey` is not used.
    func saveKeys(accessKey: String, secretKey: String, for provider: BackupProvider) throws {
        var access = accessKey.trimmingCharacters(in: .whitespacesAndNewlines)
        var secret = secretKey.trimmingCharacters(in: .whitespacesAndNewlines)
        if provider == .azure {
            access = ""
            // The Azure website shows the token with a "?" in front on some pages.
            if secret.hasPrefix("?") { secret.removeFirst() }
        } else if access.isEmpty {
            throw AWSError.missingKeys
        }
        if secret.isEmpty { throw AWSError.missingKeys }
        try keychain.save(AWSCredentials(accessKey: access, secretKey: secret), for: provider)
    }

    /// The Google page where the user signs in.
    func googleSignInURL(_ config: AWSConfig) throws -> URL {
        try googleAuth.signInURL(config)
    }

    /// Takes the address Google sent the user back to, and saves the sign-in in the Keychain.
    func finishGoogleSignIn(_ callback: URL, _ config: AWSConfig) async throws {
        let refreshToken = try await googleAuth.finishSignIn(callback, config)
        try keychain.save(AWSCredentials(accessKey: "", secretKey: refreshToken), for: .googleDrive)
    }

    func removeKeys(_ provider: BackupProvider) {
        keychain.delete(provider)
    }

    /// Asks the provider for the file list. Returns how many files are there. Throws when the settings or keys are wrong.
    func testConnection(_ config: AWSConfig) async throws -> Int {
        try await repository(config).list(try connect(config)).count
    }

    private func connect(_ config: AWSConfig) throws -> AWSConnection {
        try config.validate()
        guard let credentials = keychain.load(config.provider) else { throw AWSError.noCredentials }
        return AWSConnection(config: config, credentials: credentials)
    }

    private func repository(_ config: AWSConfig) -> any BackupRepository {
        switch config.provider {
        case .s3, .gcs: s3
        case .azure: azure
        case .googleDrive: google
        }
    }
}
