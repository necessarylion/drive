import Foundation

nonisolated struct AWSCredentials: Sendable, Equatable, Codable {
    let accessKey: String
    let secretKey: String
    var sessionToken: String?
}

/// Everything one S3 request needs to know.
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
    /// Objects in S3 that have no file on this Mac.
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
        case .notConfigured: "Set the S3 bucket first. Open Drive > Settings."
        case .badSettings(let message): message
        case .noCredentials: "No AWS access keys found. Add them in Drive > Settings."
        case .missingKeys: "Fill in both the access key ID and the secret access key."
        case .keychainFailed(let message): "The keys could not be saved in the Keychain. \(message)"
        case .requestFailed(let status, let message): "S3 said no (\(status)). \(message)"
        }
    }
}
