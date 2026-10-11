import Foundation

/// The S3 backup settings. The user edits them in the Settings window.
nonisolated struct AWSConfig: Sendable {
    var bucket = ""
    var region = AppConstants.awsDefaultRegion
    var prefix = ""

    static func saved() -> AWSConfig {
        let defaults = UserDefaults.standard
        var config = AWSConfig()
        config.bucket = clean(defaults.string(forKey: AppConstants.awsBucketKey)) ?? config.bucket
        config.region = clean(defaults.string(forKey: AppConstants.awsRegionKey)) ?? config.region
        config.prefix = clean(defaults.string(forKey: AppConstants.awsPrefixKey)) ?? config.prefix
        return config
    }

    var host: String { "\(bucket).s3.\(region).amazonaws.com" }

    /// "" for the top of the bucket, or "folder/" for a folder in it.
    var keyPrefix: String {
        let trimmed = prefix.trimmingCharacters(in: CharacterSet(charactersIn: "/ "))
        return trimmed.isEmpty ? "" : trimmed + "/"
    }

    /// The bucket and region become part of the server address, so only safe letters are allowed.
    func validate() throws {
        if bucket.isEmpty { throw AWSError.notConfigured }
        // ponytail: bucket names with dots are refused. They need another address style. Add it if you have one.
        if bucket.wholeMatch(of: /[a-z0-9][a-z0-9-]{1,61}[a-z0-9]/) == nil {
            throw AWSError.badSettings("The bucket name can only have small letters, numbers and “-”.")
        }
        if region.wholeMatch(of: /[a-z0-9-]+/) == nil {
            throw AWSError.badSettings("The region looks wrong. An example is “ap-southeast-1”.")
        }
    }

    private static func clean(_ text: String?) -> String? {
        guard let trimmed = text?.trimmingCharacters(in: .whitespacesAndNewlines), !trimmed.isEmpty else { return nil }
        return trimmed
    }
}
