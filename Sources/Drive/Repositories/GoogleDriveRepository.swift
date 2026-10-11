import Foundation

/// All network calls to Google Drive.
///
/// Google Drive has no paths. Each file has an ID and a parent folder. So `list` reads all files once
/// and builds the paths, and the other calls use the IDs it found.
final class GoogleDriveRepository: BackupRepository {
    private static let api = "https://www.googleapis.com/drive/v3/files"
    private static let uploadAPI = "https://www.googleapis.com/upload/drive/v3/files"
    private static let listFields = "nextPageToken,files(id,name,parents,mimeType,size,modifiedTime)"

    private let auth: GoogleAuthRepository
    private let session: URLSession
    /// Path -> Drive ID, from the last `list`. The path "" is the backup folder itself.
    private var folderIDs: [String: String] = [:]
    private var fileIDs: [String: String] = [:]
    /// The account and backup folder that the two maps above belong to.
    private var listedFor = ""

    init(auth: GoogleAuthRepository, session: URLSession = .shared) {
        self.auth = auth
        self.session = session
    }

    // ponytail: this reads every file the app ever made in this Google account, also the ones in another
    // backup folder. Ask Google for one folder at a time if that list gets slow.
    func list(_ connection: AWSConnection) async throws -> [S3Object] {
        var files: [GoogleFile] = []
        var pageToken: String?
        repeat {
            var address = "\(Self.api)?pageSize=1000&q=\(encode("trashed = false"))&fields=\(encode(Self.listFields))"
            if let pageToken { address += "&pageToken=\(encode(pageToken))" }
            let data = try await send(try await request("GET", address, connection))
            let page = try JSONDecoder().decode(GoogleFileList.self, from: data)
            files += page.files
            pageToken = page.nextPageToken
        } while pageToken != nil
        listedFor = tag(connection)
        return index(files, connection.config)
    }

    // ponytail: the whole file goes up in one request. Send it in parts if uploads of big files often break.
    func upload(_ file: URL, key: String, _ connection: AWSConnection) async throws {
        try await prepare(connection)
        // Step 1: tell Google what is coming. It answers with the address to send the file to.
        var start: URLRequest
        if let id = fileIDs[key] {
            start = try await request("PATCH", "\(Self.uploadAPI)/\(id)?uploadType=resumable", connection)
            start.httpBody = Data("{}".utf8)
        } else {
            let parts = key.split(separator: "/")
            let folder = try await folderID(parts.dropLast().joined(separator: "/"), connection)
            start = try await request("POST", "\(Self.uploadAPI)?uploadType=resumable", connection)
            start.httpBody = try JSONSerialization.data(withJSONObject: [
                "name": String(parts.last ?? ""), "parents": [folder],
            ])
        }
        start.setValue("application/json; charset=UTF-8", forHTTPHeaderField: "Content-Type")
        let (data, response) = try await session.data(for: start)
        try checkJSON(response, data)
        let location = (response as? HTTPURLResponse)?.value(forHTTPHeaderField: "Location") ?? ""
        guard let url = URL(string: location) else {
            throw AWSError.requestFailed(0, "Google did not say where to send the file.")
        }

        // Step 2: send the file.
        var put = URLRequest(url: url)
        put.httpMethod = "PUT"
        let (body, putResponse) = try await session.upload(for: put, fromFile: file)
        try checkJSON(putResponse, body)
        fileIDs[key] = try JSONDecoder().decode(GoogleFile.self, from: body).id
    }

    func download(key: String, _ connection: AWSConnection) async throws -> URL {
        let id = try await fileID(key, connection)
        let request = try await request("GET", "\(Self.api)/\(id)?alt=media", connection)
        let (file, response) = try await session.download(for: request)
        try checkJSON(response, (try? Data(contentsOf: file)) ?? Data())
        return file
    }

    /// Moves the file to the trash of Google Drive. Google removes it for good after 30 days.
    func delete(key: String, _ connection: AWSConnection) async throws {
        let id = try await fileID(key, connection)
        var request = try await request("PATCH", "\(Self.api)/\(id)", connection)
        request.setValue("application/json; charset=UTF-8", forHTTPHeaderField: "Content-Type")
        request.httpBody = Data(#"{"trashed": true}"#.utf8)
        _ = try await send(request)
        fileIDs[key] = nil
    }

    /// Builds the paths from the flat file list of Google, and remembers the ID of each path.
    /// Returns the files that are in the backup folder, under the configured folder in it.
    func index(_ files: [GoogleFile], _ config: AWSConfig) -> [S3Object] {
        folderIDs = [:]
        fileIDs = [:]
        var byID: [String: GoogleFile] = [:]
        for file in files {
            byID[file.id] = file
        }
        // The backup folder is the folder with the right name that is not inside another folder of ours.
        // ponytail: when two folders match, the one with the smaller ID wins. Rename one if that happens.
        var rootID = ""
        for file in files where file.isFolder && file.name == config.bucket && byID[file.parent] == nil {
            if rootID.isEmpty || file.id < rootID { rootID = file.id }
        }
        if rootID.isEmpty { return [] }
        folderIDs[""] = rootID

        var objects: [S3Object] = []
        for file in files where file.id != rootID {
            guard let path = path(of: file, in: byID, rootID: rootID) else { continue }
            if file.isFolder {
                folderIDs[path] = file.id
                continue
            }
            fileIDs[path] = file.id
            if !path.hasPrefix(config.keyPrefix) { continue }
            let date = try? Date(
                file.modifiedTime ?? "", strategy: Date.ISO8601FormatStyle(includingFractionalSeconds: true))
            objects.append(
                S3Object(key: path, size: Int64(file.size ?? "") ?? 0, lastModified: date ?? .distantPast))
        }
        return objects
    }

    /// "Docs/a.txt" for a file in the folder Docs of the backup folder. Nil when the file is not in the backup folder.
    private func path(of file: GoogleFile, in byID: [String: GoogleFile], rootID: String) -> String? {
        var names = [file.name]
        var parent = file.parent
        while parent != rootID {
            guard let folder = byID[parent] else { return nil }
            names.insert(folder.name, at: 0)
            parent = folder.parent
        }
        return names.joined(separator: "/")
    }

    /// The ID of the folder at this path. Makes the folder, and the folders above it, when they are missing.
    private func folderID(_ path: String, _ connection: AWSConnection) async throws -> String {
        if let id = folderIDs[path] { return id }
        var body: [String: Any] = ["mimeType": GoogleFile.folderType]
        if path.isEmpty {
            body["name"] = connection.config.bucket
        } else {
            let parts = path.split(separator: "/")
            body["name"] = String(parts.last ?? "")
            body["parents"] = [try await folderID(parts.dropLast().joined(separator: "/"), connection)]
        }
        var request = try await request("POST", Self.api, connection)
        request.setValue("application/json; charset=UTF-8", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONSerialization.data(withJSONObject: body)
        let id = try JSONDecoder().decode(GoogleFile.self, from: try await send(request)).id
        folderIDs[path] = id
        return id
    }

    private func fileID(_ key: String, _ connection: AWSConnection) async throws -> String {
        try await prepare(connection)
        guard let id = fileIDs[key] else {
            throw AWSError.requestFailed(404, "“\(key)” is not in Google Drive any more.")
        }
        return id
    }

    /// The IDs are only right for the account and folder that were listed. List again when they changed.
    private func prepare(_ connection: AWSConnection) async throws {
        if listedFor != tag(connection) { _ = try await list(connection) }
    }

    private func tag(_ connection: AWSConnection) -> String {
        "\(connection.credentials.secretKey)\n\(connection.config.bucket)"
    }

    private func request(_ method: String, _ address: String, _ connection: AWSConnection) async throws -> URLRequest {
        guard let url = URL(string: address) else {
            throw AWSError.badSettings("The Google Drive address could not be built.")
        }
        var request = URLRequest(url: url)
        request.httpMethod = method
        request.setValue("Bearer \(try await auth.accessToken(connection))", forHTTPHeaderField: "Authorization")
        return request
    }

    private func send(_ request: URLRequest) async throws -> Data {
        let (data, response) = try await session.data(for: request)
        try checkJSON(response, data)
        return data
    }

    private func encode(_ text: String) -> String {
        text.addingPercentEncoding(withAllowedCharacters: .alphanumerics) ?? ""
    }

    /// Google sends its errors as JSON, not as XML like the other providers.
    private func checkJSON(_ response: URLResponse, _ body: Data) throws {
        let status = (response as? HTTPURLResponse)?.statusCode ?? 0
        if (200..<300).contains(status) { return }
        let json = try? JSONSerialization.jsonObject(with: body) as? [String: Any]
        let message = (json?["error"] as? [String: Any])?["message"] as? String ?? "No details."
        throw AWSError.requestFailed(status, message)
    }
}
