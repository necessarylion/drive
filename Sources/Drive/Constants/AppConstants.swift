import Foundation

nonisolated enum AppConstants {
    static let recentLimit = 50
    static let menuBarRecentLimit = 5

    static let backupProviderKey = "backup.provider"
    static let azureAccountKey = "azure.account"
    static let googleClientIDKey = "google.clientID"
    static let awsRegionKey = "aws.region"
    static let awsPausedKey = "aws.paused"
    static let awsKeychainService = "com.zinkyaw.drive.aws"
    static let awsDefaultRegion = "us-east-1"

    static let defaultFolderName = "Untitled Folder"
    static let thumbnailSize = CGSize(width: 320, height: 240)
    static let tileMinWidth: CGFloat = 170
    static let columnWidth: CGFloat = 240
    static let dateColumnWidth: CGFloat = 140
    static let sizeColumnWidth: CGFloat = 80
    static let badgeColumnWidth: CGFloat = 28
    static let searchFieldWidth: CGFloat = 220

    static let viewModeKey = "viewMode"
    static let storageRootKey = "storage.root"
    static let mainWindowID = "main"
    static let viewerWindowID = "viewer"

    // Each provider has its own bucket and folder. S3 is "aws" here, so it keeps the keys from the time
    // when all providers shared them: "aws.bucket" and "aws.prefix".
    static func bucketKey(_ provider: BackupProvider) -> String { "\(provider.rawValue).bucket" }
    static func prefixKey(_ provider: BackupProvider) -> String { "\(provider.rawValue).prefix" }
}
