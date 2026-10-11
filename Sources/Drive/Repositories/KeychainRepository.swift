import Foundation
import Security

/// Keeps the backup keys in the Mac's Keychain, so they are never in a plain file of ours.
/// Each provider has its own keys.
final class KeychainRepository {
    private let service: String

    init(service: String) {
        self.service = service
    }

    func load(_ provider: BackupProvider) -> AWSCredentials? {
        var query = baseQuery(provider)
        query[kSecReturnData] = true
        query[kSecMatchLimit] = kSecMatchLimitOne
        var result: CFTypeRef?
        guard SecItemCopyMatching(query as CFDictionary, &result) == errSecSuccess, let data = result as? Data else {
            return nil
        }
        return try? JSONDecoder().decode(AWSCredentials.self, from: data)
    }

    func save(_ credentials: AWSCredentials, for provider: BackupProvider) throws {
        delete(provider)
        var item = baseQuery(provider)
        item[kSecValueData] = try JSONEncoder().encode(credentials)
        let status = SecItemAdd(item as CFDictionary, nil)
        if status != errSecSuccess {
            throw AWSError.keychainFailed(SecCopyErrorMessageString(status, nil) as String? ?? "Error \(status)")
        }
    }

    func delete(_ provider: BackupProvider) {
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
