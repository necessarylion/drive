import Foundation

/// The backup settings. The user edits them in the Settings window.
nonisolated struct AWSConfig: Equatable, Sendable {
    var provider = BackupProvider.s3
    /// The bucket. For Azure, this is the container. For Google Drive, this is the name of the backup folder.
    var bucket = ""
    /// The Azure storage account. Other providers do not use it.
    var account = ""
    /// The OAuth client ID of the Google Cloud project. Only Google Drive uses it. It is not a secret.
    var clientID = ""
    var region = AppConstants.awsDefaultRegion
    var prefix = ""

    /// The settings the backup uses now.
    static func saved(in defaults: UserDefaults = .standard) -> AWSConfig {
        saved(for: savedProvider(in: defaults), in: defaults)
    }

    /// The settings of one provider. Each provider remembers its own bucket and folder.
    static func saved(for provider: BackupProvider, in defaults: UserDefaults = .standard) -> AWSConfig {
        var config = AWSConfig()
        config.provider = provider
        config.account = clean(defaults.string(forKey: AppConstants.azureAccountKey)) ?? config.account
        config.clientID = clean(defaults.string(forKey: AppConstants.googleClientIDKey)) ?? config.clientID
        config.bucket = clean(defaults.string(forKey: AppConstants.bucketKey(provider))) ?? config.bucket
        config.region = clean(defaults.string(forKey: AppConstants.awsRegionKey)) ?? config.region
        config.prefix = clean(defaults.string(forKey: AppConstants.prefixKey(provider))) ?? config.prefix
        return config
    }

    /// Makes these settings the ones the backup uses.
    func save(in defaults: UserDefaults = .standard) {
        defaults.set(provider.rawValue, forKey: AppConstants.backupProviderKey)
        defaults.set(bucket, forKey: AppConstants.bucketKey(provider))
        defaults.set(prefix, forKey: AppConstants.prefixKey(provider))
        // Write only what this provider uses, or saving one provider resets a field of another one.
        switch provider {
        case .s3: defaults.set(region, forKey: AppConstants.awsRegionKey)
        case .azure: defaults.set(account, forKey: AppConstants.azureAccountKey)
        case .googleDrive: defaults.set(clientID, forKey: AppConstants.googleClientIDKey)
        case .gcs: break
        }
    }

    /// Older versions kept one bucket and one folder for all providers, under the keys that S3 has now.
    /// This gives them to the provider that was in use. Run it one time at the start of the app.
    static func migrate(in defaults: UserDefaults = .standard) {
        let provider = savedProvider(in: defaults)
        let oldBucketKey = AppConstants.bucketKey(.s3)
        let oldPrefixKey = AppConstants.prefixKey(.s3)
        // A provider that has its own bucket was saved by this version. Then the S3 keys belong to S3.
        if provider == .s3 || defaults.object(forKey: AppConstants.bucketKey(provider)) != nil { return }
        guard let bucket = defaults.string(forKey: oldBucketKey) else { return }
        defaults.set(bucket, forKey: AppConstants.bucketKey(provider))
        defaults.set(defaults.string(forKey: oldPrefixKey) ?? "", forKey: AppConstants.prefixKey(provider))
        defaults.removeObject(forKey: oldBucketKey)
        defaults.removeObject(forKey: oldPrefixKey)
    }

    private static func savedProvider(in defaults: UserDefaults) -> BackupProvider {
        BackupProvider(rawValue: defaults.string(forKey: AppConstants.backupProviderKey) ?? "") ?? .s3
    }

    var host: String {
        switch provider {
        case .s3: "\(bucket).s3.\(region).amazonaws.com"
        // Google Cloud Storage answers S3 requests at this address.
        case .gcs: "\(bucket).storage.googleapis.com"
        case .azure: "\(account).blob.core.windows.net"
        case .googleDrive: "www.googleapis.com"
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
        case .googleDrive: "Google Drive/\(bucket)/\(keyPrefix)"
        }
    }

    /// Where Google sends the user after the sign-in. Google wants the client ID turned around for it:
    /// "123.apps.googleusercontent.com" becomes "com.googleusercontent.apps.123".
    var googleRedirectScheme: String { clientID.split(separator: ".").reversed().joined(separator: ".") }
    var googleRedirectURI: String { "\(googleRedirectScheme):/oauth2redirect" }

    /// "" for the top of the bucket, or "folder/" for a folder in it.
    var keyPrefix: String {
        let trimmed = prefix.trimmingCharacters(in: CharacterSet(charactersIn: "/ "))
        return trimmed.isEmpty ? "" : trimmed + "/"
    }

    /// The bucket, region and account become part of the server address, so only safe letters are allowed.
    func validate() throws {
        if bucket.isEmpty { throw AWSError.notConfigured }
        if provider == .googleDrive {
            // The folder name is never part of an address, so any name is fine. The client ID is.
            if clientID.wholeMatch(of: /[a-z0-9-]+\.apps\.googleusercontent\.com/) == nil {
                throw AWSError.badSettings("The client ID looks wrong. It ends with “.apps.googleusercontent.com”.")
            }
            return
        }
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
