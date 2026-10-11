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
        defer { keychain.delete() }
        #expect(keychain.load() == nil)

        let saved = AWSCredentials(accessKey: "AKIAEXAMPLE", secretKey: "SECRET")
        try keychain.save(saved)
        #expect(keychain.load() == saved)

        // Saving again replaces the old keys.
        let newer = AWSCredentials(accessKey: "AKIANEWER", secretKey: "SECRET2")
        try keychain.save(newer)
        #expect(keychain.load() == newer)

        keychain.delete()
        #expect(keychain.load() == nil)
    }

    @Test func refusesUnsafeBucketNames() throws {
        try AWSConfig(bucket: "my-drive-backup").validate()
        #expect(throws: AWSError.self) { try AWSConfig(bucket: "").validate() }
        #expect(throws: AWSError.self) { try AWSConfig(bucket: "evil.com/x").validate() }
        #expect(throws: AWSError.self) { try AWSConfig(bucket: "good-name", region: "us east").validate() }
    }
}
