import Foundation

/// Where the backup goes. The raw value is also the name of the keys in the Keychain.
nonisolated enum BackupProvider: String, CaseIterable, Identifiable, Sendable {
    // "aws" is the name the keys had before other providers came. Keep it, or saved keys are lost.
    case s3 = "aws"
    case gcs
    case azure
    case googleDrive = "gdrive"

    var id: String { rawValue }

    var title: String {
        switch self {
        case .s3: "AWS S3"
        case .gcs: "Google Cloud Storage"
        case .azure: "Azure Blob Storage"
        case .googleDrive: "Google Drive"
        }
    }
}

/// For Azure, `accessKey` is empty and `secretKey` holds the SAS token.
/// For Google Drive, `accessKey` is empty and `secretKey` holds the refresh token.
nonisolated struct AWSCredentials: Sendable, Equatable, Codable {
    let accessKey: String
    let secretKey: String
    var sessionToken: String?
}

/// Everything one backup request needs to know.
nonisolated struct AWSConnection: Sendable {
    let config: AWSConfig
    let credentials: AWSCredentials
}

nonisolated struct S3Object: Sendable {
    let key: String
    let size: Int64
    let lastModified: Date
}

nonisolated struct BackupUpload: Sendable {
    let file: URL
    let key: String
    /// The file's change date when the plan was made.
    let modified: Date
}

/// What a backup will do. Made first, so the user can be asked before anything is removed.
nonisolated struct BackupPlan: Sendable {
    let connection: AWSConnection
    var uploads: [BackupUpload] = []
    /// Objects in the backup that have no file on this Mac.
    var extraKeys: [String] = []
}

nonisolated enum AWSError: LocalizedError {
    case notConfigured
    case badSettings(String)
    case noCredentials
    case requestFailed(Int, String)
    case missingKeys
    case keychainFailed(String)

    var errorDescription: String? {
        switch self {
        case .notConfigured: "Set the bucket or folder first. Open Drive > Settings."
        case .badSettings(let message): message
        case .noCredentials: "No access keys found. Add them in Drive > Settings."
        case .missingKeys: "Fill in the access keys first."
        case .keychainFailed(let message): "The keys could not be saved in the Keychain. \(message)"
        case .requestFailed(let status, let message): "The backup server said no (\(status)). \(message)"
        }
    }
}
