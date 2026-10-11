import CryptoKit
import Foundation

/// Signs in to Google with OAuth 2, and gets access tokens for the Drive calls.
final class GoogleAuthRepository {
    /// With this scope, the app only sees the files it made itself. Google needs no review for it.
    private static let scope = "https://www.googleapis.com/auth/drive.file"
    private static let signInAddress = "https://accounts.google.com/o/oauth2/v2/auth"
    private static let tokenAddress = "https://oauth2.googleapis.com/token"

    /// Only used for its URL encoding.
    private let signer: AWSSigner
    private let session: URLSession
    /// Made new for each sign-in. Google checks the verifier, and we check the state.
    private var verifier = ""
    private var state = ""
    /// The last access token, and the refresh token it was made from.
    private var accessToken = ""
    private var accessTokenOwner = ""
    private var accessTokenEnd = Date.distantPast

    init(signer: AWSSigner, session: URLSession = .shared) {
        self.signer = signer
        self.session = session
    }

    /// The Google page where the user signs in.
    func signInURL(_ config: AWSConfig) throws -> URL {
        try config.validate()
        verifier = randomText()
        state = randomText()
        let challenge = base64URL(Data(SHA256.hash(data: Data(verifier.utf8))))
        let query = form([
            "client_id": config.clientID,
            "code_challenge": challenge,
            "code_challenge_method": "S256",
            "redirect_uri": config.googleRedirectURI,
            "response_type": "code",
            "scope": Self.scope,
            "state": state,
        ])
        guard let url = URL(string: "\(Self.signInAddress)?\(query)") else {
            throw AWSError.badSettings("The Google sign-in address could not be built. Check the client ID.")
        }
        return url
    }

    /// Takes the address Google sent the user back to. Returns the refresh token, which is what gets saved.
    func finishSignIn(_ callback: URL, _ config: AWSConfig) async throws -> String {
        var values: [String: String] = [:]
        for item in URLComponents(url: callback, resolvingAgainstBaseURL: false)?.queryItems ?? [] {
            values[item.name] = item.value
        }
        // The state proves that this answer belongs to the sign-in we started.
        guard let code = values["code"], !state.isEmpty, values["state"] == state else {
            throw AWSError.badSettings("Google did not finish the sign-in. \(values["error"] ?? "Try again.")")
        }
        let token = try await requestToken([
            "client_id": config.clientID,
            "code": code,
            "code_verifier": verifier,
            "grant_type": "authorization_code",
            "redirect_uri": config.googleRedirectURI,
        ])
        state = ""
        verifier = ""
        guard let refreshToken = token.refreshToken else {
            throw AWSError.badSettings("Google did not send a refresh token. Try to sign in again.")
        }
        remember(token, owner: refreshToken)
        return refreshToken
    }

    /// A token that works now. It asks Google for a new one when the old one is about to end.
    func accessToken(_ connection: AWSConnection) async throws -> String {
        let refreshToken = connection.credentials.secretKey
        if accessTokenOwner == refreshToken && accessTokenEnd > .now { return accessToken }
        let token = try await requestToken([
            "client_id": connection.config.clientID,
            "grant_type": "refresh_token",
            "refresh_token": refreshToken,
        ])
        remember(token, owner: refreshToken)
        return accessToken
    }

    private func remember(_ token: GoogleToken, owner: String) {
        accessToken = token.accessToken
        accessTokenOwner = owner
        // Stop using it one minute early, so it does not end in the middle of a request.
        accessTokenEnd = Date.now.addingTimeInterval(token.expiresIn - 60)
    }

    private func requestToken(_ fields: [String: String]) async throws -> GoogleToken {
        guard let url = URL(string: Self.tokenAddress) else { throw AWSError.badSettings("Bad token address.") }
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/x-www-form-urlencoded", forHTTPHeaderField: "Content-Type")
        request.httpBody = Data(form(fields).utf8)
        let (data, response) = try await session.data(for: request)
        let status = (response as? HTTPURLResponse)?.statusCode ?? 0
        if !(200..<300).contains(status) {
            let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
            let reason = json?["error_description"] as? String ?? json?["error"] as? String ?? "No details."
            throw AWSError.badSettings("Google refused the sign-in. Sign in again in Drive > Settings. (\(reason))")
        }
        return try JSONDecoder().decode(GoogleToken.self, from: data)
    }

    /// "a=1&b=2", with the names in order and the values encoded.
    private func form(_ fields: [String: String]) -> String {
        var pairs: [String] = []
        for name in fields.keys.sorted() {
            pairs.append("\(name)=\(signer.uriEncode(fields[name, default: ""], keepSlash: false))")
        }
        return pairs.joined(separator: "&")
    }

    private func randomText() -> String {
        var bytes: [UInt8] = []
        for _ in 0..<32 {
            // The system random numbers are safe to use for secrets.
            bytes.append(UInt8.random(in: .min ... .max))
        }
        return base64URL(Data(bytes))
    }

    private func base64URL(_ data: Data) -> String {
        data.base64EncodedString()
            .replacingOccurrences(of: "+", with: "-")
            .replacingOccurrences(of: "/", with: "_")
            .replacingOccurrences(of: "=", with: "")
    }
}
