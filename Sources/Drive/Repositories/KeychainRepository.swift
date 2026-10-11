import Foundation
import Security

/// Keeps the AWS keys in the Mac's Keychain, so they are never in a plain file of ours.
final class KeychainRepository {
    private let service: String

    init(service: String) {
        self.service = service
    }

    func load() -> AWSCredentials? {
        var query = baseQuery()
        query[kSecReturnData] = true
        query[kSecMatchLimit] = kSecMatchLimitOne
        var result: CFTypeRef?
        guard SecItemCopyMatching(query as CFDictionary, &result) == errSecSuccess, let data = result as? Data else {
            return nil
        }
        return try? JSONDecoder().decode(AWSCredentials.self, from: data)
    }

    func save(_ credentials: AWSCredentials) throws {
        delete()
        var item = baseQuery()
        item[kSecValueData] = try JSONEncoder().encode(credentials)
        let status = SecItemAdd(item as CFDictionary, nil)
        if status != errSecSuccess {
            throw AWSError.keychainFailed(SecCopyErrorMessageString(status, nil) as String? ?? "Error \(status)")
        }
    }

    func delete() {
        SecItemDelete(baseQuery() as CFDictionary)
    }

    private func baseQuery() -> [CFString: Any] {
        [
            kSecClass: kSecClassGenericPassword,
            kSecAttrService: service,
            kSecAttrAccount: "aws",
        ]
    }
}
