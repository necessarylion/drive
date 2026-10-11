import Foundation

/// All network calls to Azure Blob Storage.
// ponytail: it signs in with a SAS token, and that token has an end date.
// Sign each request with the account key if making new tokens gets annoying.
final class AzureRepository: BackupRepository {
    /// The version of the Azure API these calls are written for.
    private static let apiVersion = "2021-08-06"

    /// Only used for its URL encoding.
    private let signer: AWSSigner
    private let session: URLSession

    init(signer: AWSSigner, session: URLSession = .shared) {
        self.signer = signer
        self.session = session
    }

    func list(_ connection: AWSConnection) async throws -> [S3Object] {
        var objects: [S3Object] = []
        var marker: String?
        repeat {
            var query = ["restype": "container", "comp": "list", "prefix": connection.config.keyPrefix]
            if let marker { query["marker"] = marker }
            let (data, response) = try await session.data(for: request("GET", key: "", query: query, connection))
            try check(response, data)
            let page = try page(from: data)
            objects += page.objects
            marker = page.marker
        } while marker != nil
        return objects
    }

    // ponytail: one PUT per file, so about 5 GB is the biggest file. Upload in blocks if you need more.
    func upload(_ file: URL, key: String, _ connection: AWSConnection) async throws {
        var request = try request("PUT", key: key, connection)
        request.setValue("BlockBlob", forHTTPHeaderField: "x-ms-blob-type")
        let (data, response) = try await session.upload(for: request, fromFile: file)
        try check(response, data)
    }

    func download(key: String, _ connection: AWSConnection) async throws -> URL {
        let (file, response) = try await session.download(for: request("GET", key: key, connection))
        try check(response, (try? Data(contentsOf: file)) ?? Data())
        return file
    }

    func delete(key: String, _ connection: AWSConnection) async throws {
        let (data, response) = try await session.data(for: request("DELETE", key: key, connection))
        try check(response, data)
    }

    /// Reads one page of the blob list. `marker` is nil on the last page.
    func page(from data: Data) throws -> (objects: [S3Object], marker: String?) {
        let root = try XMLDocument(data: data).rootElement()
        let formatter = DateFormatter()
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.timeZone = TimeZone(identifier: "UTC")
        formatter.dateFormat = "EEE, dd MMM yyyy HH:mm:ss 'GMT'"

        var objects: [S3Object] = []
        for blob in root?.elements(forName: "Blobs").first?.elements(forName: "Blob") ?? [] {
            let properties = blob.elements(forName: "Properties").first
            // A storage account with real folders also lists the folders. They are not files.
            if properties?.elements(forName: "ResourceType").first?.stringValue == "directory" { continue }
            let dateText = properties?.elements(forName: "Last-Modified").first?.stringValue ?? ""
            objects.append(
                S3Object(
                    key: blob.elements(forName: "Name").first?.stringValue ?? "",
                    size: Int64(properties?.elements(forName: "Content-Length").first?.stringValue ?? "") ?? 0,
                    lastModified: formatter.date(from: dateText) ?? .distantPast))
        }
        let marker = root?.elements(forName: "NextMarker").first?.stringValue ?? ""
        return (objects, marker.isEmpty ? nil : marker)
    }

    /// The SAS token goes at the end of the address. It is the sign-in, so no header is signed.
    func request(
        _ method: String, key: String, query: [String: String] = [:], _ connection: AWSConnection
    ) throws -> URLRequest {
        let config = connection.config
        var path = "/\(config.bucket)"
        if !key.isEmpty { path += "/" + signer.uriEncode(key, keepSlash: true) }
        var pairs: [String] = []
        for name in query.keys.sorted() {
            pairs.append("\(name)=\(signer.uriEncode(query[name, default: ""], keepSlash: false))")
        }
        // The token is already encoded by Azure.
        pairs.append(connection.credentials.secretKey)

        guard let url = URL(string: "https://\(config.host)\(path)?\(pairs.joined(separator: "&"))") else {
            throw AWSError.badSettings("The address could not be built. Check the account, container and SAS token.")
        }
        var request = URLRequest(url: url)
        request.httpMethod = method
        request.setValue(Self.apiVersion, forHTTPHeaderField: "x-ms-version")
        return request
    }
}
