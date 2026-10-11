import Foundation
import Security

/// Keeps the backup keys in the Mac's Keychain, so they are never in a plain file of ours.
/// Each provider has its own keys.
final class KeychainRepository {
    private let service: String
    /// The keys we already read. macOS asks the user on every read of the Keychain when they
    /// press "Allow" and not "Always Allow", so we read each key only one time per launch.
    private var cache: [BackupProvider: AWSCredentials] = [:]

    init(service: String) {
        self.service = service
    }

    func load(_ provider: BackupProvider) -> AWSCredentials? {
        if let cached = cache[provider] { return cached }
        var query = baseQuery(provider)
        query[kSecReturnData] = true
        query[kSecMatchLimit] = kSecMatchLimitOne
        var result: CFTypeRef?
        guard SecItemCopyMatching(query as CFDictionary, &result) == errSecSuccess, let data = result as? Data else {
            return nil
        }
        let credentials = try? JSONDecoder().decode(AWSCredentials.self, from: data)
        cache[provider] = credentials
        return credentials
    }

    func save(_ credentials: AWSCredentials, for provider: BackupProvider) throws {
        delete(provider)
        var item = baseQuery(provider)
        item[kSecValueData] = try JSONEncoder().encode(credentials)
        let status = SecItemAdd(item as CFDictionary, nil)
        if status != errSecSuccess {
            throw AWSError.keychainFailed(SecCopyErrorMessageString(status, nil) as String? ?? "Error \(status)")
        }
        cache[provider] = credentials
    }

    func delete(_ provider: BackupProvider) {
        cache[provider] = nil
        SecItemDelete(baseQuery(provider) as CFDictionary)
    }

    private func baseQuery(_ provider: BackupProvider) -> [CFString: Any] {
        [
            kSecClass: kSecClassGenericPassword,
            kSecAttrService: service,
            kSecAttrAccount: provider.rawValue,
        ]
    }
}
