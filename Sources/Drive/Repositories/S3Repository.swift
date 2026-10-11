import Foundation

/// All network calls to S3. Google Cloud Storage speaks the same language, so it is used for that too.
final class S3Repository: BackupRepository {
    private let signer: AWSSigner
    private let session: URLSession

    init(signer: AWSSigner, session: URLSession = .shared) {
        self.signer = signer
        self.session = session
    }

    /// Every object under the configured folder of the bucket.
    func list(_ connection: AWSConnection) async throws -> [S3Object] {
        var objects: [S3Object] = []
        var token: String?
        repeat {
            var query = ["list-type": "2", "prefix": connection.config.keyPrefix]
            if let token { query["continuation-token"] = token }
            let (data, response) = try await session.data(for: request("GET", key: "", query: query, connection))
            try check(response, data)
            let root = try XMLDocument(data: data).rootElement()
            for node in root?.elements(forName: "Contents") ?? [] {
                let dateText = node.elements(forName: "LastModified").first?.stringValue ?? ""
                let date = try? Date(dateText, strategy: Date.ISO8601FormatStyle(includingFractionalSeconds: true))
                objects.append(
                    S3Object(
                        key: node.elements(forName: "Key").first?.stringValue ?? "",
                        size: Int64(node.elements(forName: "Size").first?.stringValue ?? "") ?? 0,
                        lastModified: date ?? .distantPast))
            }
            token = root?.elements(forName: "NextContinuationToken").first?.stringValue
        } while token != nil
        return objects
    }

    // ponytail: one PUT per file, so 5 GB is the biggest file. Use multipart upload if you need more.
    func upload(_ file: URL, key: String, _ connection: AWSConnection) async throws {
        let (data, response) = try await session.upload(for: request("PUT", key: key, connection), fromFile: file)
        try check(response, data)
    }

    /// Downloads to a temporary file and returns where it is.
    func download(key: String, _ connection: AWSConnection) async throws -> URL {
        let (file, response) = try await session.download(for: request("GET", key: key, connection))
        try check(response, (try? Data(contentsOf: file)) ?? Data())
        return file
    }

    // ponytail: one request per object. Use the batch delete call if removing thousands gets slow.
    func delete(key: String, _ connection: AWSConnection) async throws {
        let (data, response) = try await session.data(for: request("DELETE", key: key, connection))
        try check(response, data)
    }

    private func request(
        _ method: String, key: String, query: [String: String] = [:], _ connection: AWSConnection
    ) throws -> URLRequest {
        let config = connection.config
        let path = "/" + signer.uriEncode(key, keepSlash: true)
        var pairs: [String] = []
        for name in query.keys.sorted() {
            let value = signer.uriEncode(query[name, default: ""], keepSlash: false)
            pairs.append("\(signer.uriEncode(name, keepSlash: false))=\(value)")
        }
        let queryString = pairs.joined(separator: "&")

        var headers = [
            "host": config.host,
            "x-amz-content-sha256": AWSSigner.unsignedPayload,
            "x-amz-date": timestamp(),
        ]
        if let token = connection.credentials.sessionToken { headers["x-amz-security-token"] = token }
        let authorization = signer.authorization(
            method: method, path: path, query: queryString, headers: headers, payloadHash: AWSSigner.unsignedPayload,
            region: config.signingRegion, credentials: connection.credentials)

        let address = "https://\(config.host)\(path)" + (queryString.isEmpty ? "" : "?\(queryString)")
        guard let url = URL(string: address) else {
            throw AWSError.badSettings("The address could not be built. Check the bucket and region.")
        }
        var request = URLRequest(url: url)
        request.httpMethod = method
        // URLSession sets Host itself, from the URL.
        for (name, value) in headers where name != "host" {
            request.setValue(value, forHTTPHeaderField: name)
        }
        request.setValue(authorization, forHTTPHeaderField: "Authorization")
        return request
    }

    private func timestamp() -> String {
        let formatter = DateFormatter()
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.timeZone = TimeZone(identifier: "UTC")
        formatter.dateFormat = "yyyyMMdd'T'HHmmss'Z'"
        return formatter.string(from: .now)
    }
}
