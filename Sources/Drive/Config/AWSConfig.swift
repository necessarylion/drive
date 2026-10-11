import Foundation

/// The backup settings. The user edits them in the Settings window.
nonisolated struct AWSConfig: Sendable {
    var provider = BackupProvider.s3
    /// The bucket. For Azure, this is the container.
    var bucket = ""
    /// The Azure storage account. Other providers do not use it.
    var account = ""
    var region = AppConstants.awsDefaultRegion
    var prefix = ""

    static func saved() -> AWSConfig {
        let defaults = UserDefaults.standard
        var config = AWSConfig()
        config.provider = BackupProvider(rawValue: defaults.string(forKey: AppConstants.backupProviderKey) ?? "") ?? .s3
        config.account = clean(defaults.string(forKey: AppConstants.azureAccountKey)) ?? config.account
        config.bucket = clean(defaults.string(forKey: AppConstants.awsBucketKey)) ?? config.bucket
        config.region = clean(defaults.string(forKey: AppConstants.awsRegionKey)) ?? config.region
        config.prefix = clean(defaults.string(forKey: AppConstants.awsPrefixKey)) ?? config.prefix
        return config
    }

    var host: String {
        switch provider {
        case .s3: "\(bucket).s3.\(region).amazonaws.com"
        // Google Cloud Storage answers S3 requests at this address.
        case .gcs: "\(bucket).storage.googleapis.com"
        case .azure: "\(account).blob.core.windows.net"
        }
    }

    /// The region that goes into the request signature. Google has no regions here and takes "auto".
    var signingRegion: String { provider == .gcs ? "auto" : region }

    /// Where the backup goes, like "s3://bucket/folder/".
    var destination: String {
        switch provider {
        case .s3: "s3://\(bucket)/\(keyPrefix)"
        case .gcs: "gs://\(bucket)/\(keyPrefix)"
        case .azure: "https://\(host)/\(bucket)/\(keyPrefix)"
        }
    }

    /// "" for the top of the bucket, or "folder/" for a folder in it.
    var keyPrefix: String {
        let trimmed = prefix.trimmingCharacters(in: CharacterSet(charactersIn: "/ "))
        return trimmed.isEmpty ? "" : trimmed + "/"
    }

    /// The bucket, region and account become part of the server address, so only safe letters are allowed.
    func validate() throws {
        if bucket.isEmpty { throw AWSError.notConfigured }
        // ponytail: bucket names with dots or “_” are refused. They need another address style. Add it if you have one.
        if bucket.wholeMatch(of: /[a-z0-9][a-z0-9-]{1,61}[a-z0-9]/) == nil {
            throw AWSError.badSettings("The bucket name can only have small letters, numbers and “-”.")
        }
        if provider == .azure && account.wholeMatch(of: /[a-z0-9]{3,24}/) == nil {
            throw AWSError.badSettings("The storage account name can only have small letters and numbers.")
        }
        if provider == .s3 && region.wholeMatch(of: /[a-z0-9-]+/) == nil {
            throw AWSError.badSettings("The region looks wrong. An example is “ap-southeast-1”.")
        }
    }

    private static func clean(_ text: String?) -> String? {
        guard let trimmed = text?.trimmingCharacters(in: .whitespacesAndNewlines), !trimmed.isEmpty else { return nil }
        return trimmed
    }
}
