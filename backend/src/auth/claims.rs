use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivyClaims {
    aud: String, // App ID
    pub exp: u64,    // Expiration timestamp
    iss: String, // Issuer
    pub sub: String, // User ID (Privy DID)
    sid: String, // Session ID
    iat: u64,    // Issued at timestamp
}
