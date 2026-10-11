import Foundation

nonisolated enum AppConstants {
    static let recentLimit = 50
    static let menuBarRecentLimit = 5

    static let backupProviderKey = "backup.provider"
    static let azureAccountKey = "azure.account"
    static let googleClientIDKey = "google.clientID"
    // ponytail: the keys below are shared by all providers. Give each provider its own if people switch a lot.
    static let awsBucketKey = "aws.bucket"
    static let awsRegionKey = "aws.region"
    static let awsPrefixKey = "aws.prefix"
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
}
