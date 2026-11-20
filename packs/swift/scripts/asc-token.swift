import CryptoKit
import Foundation

func fail(_ message: String) -> Never {
    FileHandle.standardError.write(Data("asc-token: \(message)\n".utf8))
    exit(2)
}

func base64url(_ data: Data) -> String {
    data.base64EncodedString()
        .replacingOccurrences(of: "+", with: "-")
        .replacingOccurrences(of: "/", with: "_")
        .replacingOccurrences(of: "=", with: "")
}

func json(_ object: [String: Any]) -> Data {
    guard let data = try? JSONSerialization.data(withJSONObject: object, options: .sortedKeys) else {
        fail("cannot encode \(object)")
    }
    return data
}

let env = ProcessInfo.processInfo.environment
guard let keyId = env["ASC_KEY_ID"], !keyId.isEmpty,
    let issuer = env["ASC_ISSUER_ID"], !issuer.isEmpty,
    let path = env["ASC_KEY_PATH"], !path.isEmpty
else {
    fail("needs ASC_KEY_ID, ASC_ISSUER_ID and ASC_KEY_PATH")
}
guard let pem = try? String(contentsOfFile: path, encoding: .utf8) else {
    fail("cannot read \(path)")
}
guard let key = try? P256.Signing.PrivateKey(pemRepresentation: pem) else {
    fail("\(path) is not a P-256 private key")
}
let now = Int(Date().timeIntervalSince1970)
let header = json(["alg": "ES256", "kid": keyId, "typ": "JWT"])
let claims = json(["iss": issuer, "iat": now, "exp": now + 1200, "aud": "appstoreconnect-v1"])
let input = base64url(header) + "." + base64url(claims)
guard let signature = try? key.signature(for: Data(input.utf8)) else {
    fail("cannot sign")
}
print(input + "." + base64url(signature.rawRepresentation))
