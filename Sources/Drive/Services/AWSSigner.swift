import CryptoKit
import Foundation

/// Signs S3 requests with AWS Signature Version 4.
final class AWSSigner {
    /// Tells S3 the body is not part of the signature. Allowed over HTTPS, and saves hashing big files.
    static let unsignedPayload = "UNSIGNED-PAYLOAD"

    /// Returns the value of the Authorization header.
    /// - Parameters:
    ///   - path: The URL path, already encoded with `uriEncode`.
    ///   - query: The query string, already encoded and sorted by name.
    ///   - headers: Every header to sign, with names in small letters. Needs "host" and "x-amz-date".
    func authorization(
        method: String, path: String, query: String, headers: [String: String], payloadHash: String, region: String,
        credentials: AWSCredentials
    ) -> String {
        let names = headers.keys.sorted()
        var canonicalHeaders = ""
        for name in names {
            canonicalHeaders += "\(name):\(headers[name, default: ""].trimmingCharacters(in: .whitespaces))\n"
        }
        let signedHeaders = names.joined(separator: ";")
        let canonicalRequest = [method, path, query, canonicalHeaders, signedHeaders, payloadHash]
            .joined(separator: "\n")

        let amzDate = headers["x-amz-date", default: ""]
        let day = String(amzDate.prefix(8))
        let scope = "\(day)/\(region)/s3/aws4_request"
        let requestHash = hex(SHA256.hash(data: Data(canonicalRequest.utf8)))
        let stringToSign = "AWS4-HMAC-SHA256\n\(amzDate)\n\(scope)\n\(requestHash)"

        var key = hmac(Data("AWS4\(credentials.secretKey)".utf8), day)
        key = hmac(key, region)
        key = hmac(key, "s3")
        key = hmac(key, "aws4_request")
        let signature = hex(hmac(key, stringToSign))

        return "AWS4-HMAC-SHA256 Credential=\(credentials.accessKey)/\(scope),SignedHeaders=\(signedHeaders),"
            + "Signature=\(signature)"
    }

    /// Percent-encodes the way AWS wants: all bytes but letters, numbers and "-_.~".
    func uriEncode(_ text: String, keepSlash: Bool) -> String {
        var encoded = ""
        for byte in text.utf8 {
            let character = Character(UnicodeScalar(byte))
            let isPlain = byte < 128 && (character.isLetter || character.isNumber || "-_.~".contains(character))
            if isPlain || (keepSlash && character == "/") {
                encoded.append(character)
            } else {
                encoded += String(format: "%%%02X", byte)
            }
        }
        return encoded
    }

    private func hmac(_ key: Data, _ message: String) -> Data {
        Data(HMAC<SHA256>.authenticationCode(for: Data(message.utf8), using: SymmetricKey(data: key)))
    }

    private func hex(_ bytes: some Sequence<UInt8>) -> String {
        var text = ""
        for byte in bytes {
            text += String(format: "%02x", byte)
        }
        return text
    }
}
