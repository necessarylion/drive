import Foundation
import Testing

@testable import Drive

@MainActor
struct AWSTests {
    private let signer = AWSSigner()
    // The example keys and results below come from the AWS Signature Version 4 documentation for S3.
    private let credentials = AWSCredentials(
        accessKey: "AKIAIOSFODNN7EXAMPLE", secretKey: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY")
    private let emptyHash = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"

    @Test func signsLikeTheAWSExamples() {
        var headers = [
            "host": "examplebucket.s3.amazonaws.com",
            "x-amz-content-sha256": emptyHash,
            "x-amz-date": "20130524T000000Z",
        ]
        let list = signer.authorization(
            method: "GET", path: "/", query: "max-keys=2&prefix=J", headers: headers, payloadHash: emptyHash,
            region: "us-east-1", credentials: credentials)
        #expect(list.hasSuffix("Signature=34b48302e7b5fa45bde8084f4b7868a86f0a534bc59db6670ed5711ef69dc6f7"))

        headers["range"] = "bytes=0-9"
        let get = signer.authorization(
            method: "GET", path: "/test.txt", query: "", headers: headers, payloadHash: emptyHash,
            region: "us-east-1", credentials: credentials)
        #expect(
            get == "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request,"
                + "SignedHeaders=host;range;x-amz-content-sha256;x-amz-date,"
                + "Signature=f0e8bdb87c964420e857bd35b5d6ed310bd44f0170aba48dd91039c6036bdb41")
    }

    @Test func encodesKeys() {
        #expect(signer.uriEncode("Docs/My file (1)+é.txt", keepSlash: true) == "Docs/My%20file%20%281%29%2B%C3%A9.txt")
        #expect(signer.uriEncode("a/b", keepSlash: false) == "a%2Fb")
    }

    @Test func keepsKeysInTheKeychain() throws {
        let keychain = KeychainRepository(service: "drive-tests-\(UUID().uuidString)")
        defer {
            keychain.delete(.s3)
            keychain.delete(.azure)
        }
        #expect(keychain.load(.s3) == nil)

        let saved = AWSCredentials(accessKey: "AKIAEXAMPLE", secretKey: "SECRET")
        try keychain.save(saved, for: .s3)
        #expect(keychain.load(.s3) == saved)

        // Each provider has its own keys.
        #expect(keychain.load(.azure) == nil)
        let token = AWSCredentials(accessKey: "", secretKey: "sv=2022&sig=abc")
        try keychain.save(token, for: .azure)
        #expect(keychain.load(.azure) == token)
        #expect(keychain.load(.s3) == saved)

        // Saving again replaces the old keys.
        let newer = AWSCredentials(accessKey: "AKIANEWER", secretKey: "SECRET2")
        try keychain.save(newer, for: .s3)
        #expect(keychain.load(.s3) == newer)

        keychain.delete(.s3)
        #expect(keychain.load(.s3) == nil)
    }

    @Test func refusesUnsafeBucketNames() throws {
        try AWSConfig(bucket: "my-drive-backup").validate()
        #expect(throws: AWSError.self) { try AWSConfig(bucket: "").validate() }
        #expect(throws: AWSError.self) { try AWSConfig(bucket: "evil.com/x").validate() }
        #expect(throws: AWSError.self) { try AWSConfig(bucket: "good-name", region: "us east").validate() }
    }

    @Test func buildsTheAddressOfEachProvider() throws {
        var config = AWSConfig(bucket: "my-backup", region: "eu-west-1", prefix: "mac")
        #expect(config.host == "my-backup.s3.eu-west-1.amazonaws.com")
        #expect(config.signingRegion == "eu-west-1")
        #expect(config.destination == "s3://my-backup/mac/")

        // Google does not use the region, so a bad one must not stop the backup.
        config.provider = .gcs
        config.region = "not used"
        try config.validate()
        #expect(config.host == "my-backup.storage.googleapis.com")
        #expect(config.signingRegion == "auto")
        #expect(config.destination == "gs://my-backup/mac/")

        config.provider = .azure
        #expect(throws: AWSError.self) { try config.validate() }
        config.account = "evil.com/x"
        #expect(throws: AWSError.self) { try config.validate() }
        config.account = "mydrive"
        try config.validate()
        #expect(config.destination == "https://mydrive.blob.core.windows.net/my-backup/mac/")
    }

    @Test func buildsTheGoogleSignInAddress() throws {
        var config = AWSConfig(provider: .googleDrive, bucket: "Drive Backup")
        // Without a good client ID there is no sign-in.
        #expect(throws: AWSError.self) { try GoogleAuthRepository(signer: signer).signInURL(config) }
        config.clientID = "evil.com/x"
        #expect(throws: AWSError.self) { try config.validate() }

        config.clientID = "123-abc.apps.googleusercontent.com"
        #expect(config.googleRedirectURI == "com.googleusercontent.apps.123-abc:/oauth2redirect")
        #expect(config.destination == "Google Drive/Drive Backup/")

        let address = try GoogleAuthRepository(signer: signer).signInURL(config).absoluteString
        #expect(address.hasPrefix("https://accounts.google.com/o/oauth2/v2/auth?client_id=123-abc.apps."))
        #expect(address.contains("&redirect_uri=com.googleusercontent.apps.123-abc%3A%2Foauth2redirect&"))
        #expect(address.contains("&scope=https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fdrive.file&"))
        #expect(address.contains("&code_challenge_method=S256&"))
    }

    @Test func refusesASignInAnswerItDidNotAskFor() async throws {
        let config = AWSConfig(
            provider: .googleDrive, bucket: "Drive Backup", clientID: "123-abc.apps.googleusercontent.com")
        let auth = GoogleAuthRepository(signer: signer)
        _ = try auth.signInURL(config)
        let callback = try #require(URL(string: "\(config.googleRedirectURI)?code=stolen&state=wrong"))
        // The state is wrong, so it stops before any network call.
        await #expect(throws: AWSError.self) { try await auth.finishSignIn(callback, config) }
    }

    @Test func buildsPathsFromTheGoogleDriveList() {
        let folder = GoogleFile.folderType
        let files = [
            GoogleFile(id: "root", name: "Drive Backup", parents: ["my-drive"], mimeType: folder),
            GoogleFile(id: "docs", name: "Docs", parents: ["root"], mimeType: folder),
            GoogleFile(
                id: "a", name: "a.txt", parents: ["docs"], mimeType: "text/plain", size: "42",
                modifiedTime: "2009-10-09T21:04:30.000Z"),
            GoogleFile(id: "b", name: "b.txt", parents: ["root"], mimeType: "text/plain", size: "7"),
            // A file of the app that is in another folder. It is not part of this backup.
            GoogleFile(id: "other", name: "Old Backup", parents: ["my-drive"], mimeType: folder),
            GoogleFile(id: "c", name: "c.txt", parents: ["other"], mimeType: "text/plain", size: "1"),
        ]
        let drive = GoogleDriveRepository(auth: GoogleAuthRepository(signer: signer))

        var config = AWSConfig(provider: .googleDrive, bucket: "Drive Backup")
        var keys: [String] = []
        for object in drive.index(files, config) {
            keys.append(object.key)
        }
        #expect(keys.sorted() == ["Docs/a.txt", "b.txt"])
        let first = drive.index(files, config).first
        #expect(first?.size == 42)
        #expect(first?.lastModified == Date(timeIntervalSince1970: 1_255_122_270))

        // Only the configured folder in the backup folder counts, like the prefix of a bucket.
        config.prefix = "Docs"
        #expect(drive.index(files, config).count == 1)

        // No backup folder yet means an empty backup.
        config.bucket = "Not There"
        #expect(drive.index(files, config).isEmpty)
    }

    @Test func buildsAzureRequests() throws {
        let azure = AzureRepository(signer: signer)
        let config = AWSConfig(provider: .azure, bucket: "my-backup", account: "mydrive")
        let connection = AWSConnection(
            config: config, credentials: AWSCredentials(accessKey: "", secretKey: "sv=2022-11-02&sig=a%2Fb"))

        let put = try azure.request("PUT", key: "Docs/My file+1.txt", connection)
        #expect(
            put.url?.absoluteString
                == "https://mydrive.blob.core.windows.net/my-backup/Docs/My%20file%2B1.txt?sv=2022-11-02&sig=a%2Fb")

        let list = try azure.request("GET", key: "", query: ["comp": "list", "prefix": "a b/"], connection)
        #expect(
            list.url?.absoluteString
                == "https://mydrive.blob.core.windows.net/my-backup?comp=list&prefix=a%20b%2F&sv=2022-11-02&sig=a%2Fb")
    }

    @Test func readsTheAzureBlobList() throws {
        let xml = """
            <?xml version="1.0" encoding="utf-8"?>
            <EnumerationResults ServiceEndpoint="https://mydrive.blob.core.windows.net/" ContainerName="my-backup">
              <Blobs>
                <Blob>
                  <Name>Docs/a.txt</Name>
                  <Properties>
                    <Last-Modified>Fri, 09 Oct 2009 21:04:30 GMT</Last-Modified>
                    <Content-Length>42</Content-Length>
                  </Properties>
                </Blob>
                <Blob>
                  <Name>Docs</Name>
                  <Properties>
                    <Last-Modified>Fri, 09 Oct 2009 21:04:30 GMT</Last-Modified>
                    <Content-Length>0</Content-Length>
                    <ResourceType>directory</ResourceType>
                  </Properties>
                </Blob>
              </Blobs>
              <NextMarker>next-page</NextMarker>
            </EnumerationResults>
            """
        let page = try AzureRepository(signer: signer).page(from: Data(xml.utf8))
        #expect(page.marker == "next-page")
        #expect(page.objects.count == 1)
        #expect(page.objects.first?.key == "Docs/a.txt")
        #expect(page.objects.first?.size == 42)
        #expect(page.objects.first?.lastModified == Date(timeIntervalSince1970: 1_255_122_270))

        let last = try AzureRepository(signer: signer).page(
            from: Data("<EnumerationResults><NextMarker /></EnumerationResults>".utf8))
        #expect(last.marker == nil)
        #expect(last.objects.isEmpty)
    }
}
