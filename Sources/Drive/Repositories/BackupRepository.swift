import Foundation

/// One place that keeps the backup. `S3Repository` and `AzureRepository` are the two kinds.
protocol BackupRepository {
    /// Every object under the configured folder.
    func list(_ connection: AWSConnection) async throws -> [S3Object]
    func upload(_ file: URL, key: String, _ connection: AWSConnection) async throws
    /// Downloads to a temporary file and returns where it is.
    func download(key: String, _ connection: AWSConnection) async throws -> URL
    func delete(key: String, _ connection: AWSConnection) async throws
}

extension BackupRepository {
    /// Throws when the server did not say OK. All three providers put the reason in a "Message" tag.
    func check(_ response: URLResponse, _ body: Data) throws {
        let status = (response as? HTTPURLResponse)?.statusCode ?? 0
        if (200..<300).contains(status) { return }
        let root = try? XMLDocument(data: body).rootElement()
        let message = root?.elements(forName: "Message").first?.stringValue ?? "No details."
        throw AWSError.requestFailed(status, message)
    }
}
