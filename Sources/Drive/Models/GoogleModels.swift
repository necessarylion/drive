import Foundation

/// One file or folder, the way the Google Drive API sends it.
nonisolated struct GoogleFile: Sendable, Decodable {
    static let folderType = "application/vnd.google-apps.folder"

    let id: String
    var name = ""
    var parents: [String]?
    var mimeType: String?
    /// The size in bytes, as text. Folders have none.
    var size: String?
    var modifiedTime: String?

    var isFolder: Bool { mimeType == Self.folderType }
    var parent: String { parents?.first ?? "" }
}

nonisolated struct GoogleFileList: Sendable, Decodable {
    let files: [GoogleFile]
    var nextPageToken: String?
}

/// The answer of the Google token address.
nonisolated struct GoogleToken: Sendable, Decodable {
    let accessToken: String
    /// Seconds until the access token stops working.
    let expiresIn: Double
    /// Only sent after a sign-in, not after a refresh.
    var refreshToken: String?

    enum CodingKeys: String, CodingKey {
        case accessToken = "access_token"
        case expiresIn = "expires_in"
        case refreshToken = "refresh_token"
    }
}
