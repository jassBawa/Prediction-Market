use std::env;

use axum::{
    body::Body,
    http::{HeaderMap, Request, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation};

use crate::auth::claims::{AuthUser, PrivyClaims, RawClaims};

// use crate::auth::privy::validate_privy_jwt;

pub async fn auth_middleware(
    headers: HeaderMap,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get("privy-id-token")
        .and_then(|h| h.to_str().ok());

    let access_token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    let token = match auth_header {
        Some(c) => c.to_string(),
        None => return Err(StatusCode::UNAUTHORIZED),
    };

    let public_key =
        env::var("PRIVY_VERIFICATION_PEM").expect("PRIVY_VERIFICAITION_PEM environment not set");
    let app_id = env::var("PRIVY_APP_ID").expect("PRIVY_APP_ID environment not set");
    let mut validation = Validation::new(jsonwebtoken::Algorithm::ES256);
    validation.set_issuer(&["privy.io"]);
    validation.set_audience(&[&app_id]);

    let decoding_key = DecodingKey::from_ec_pem(public_key.as_bytes()).expect("INVALID public key");

    let token_data =
        decode::<RawClaims>(token, &decoding_key, &validation).expect("Token verifction failed");

    let data = PrivyClaims {
        aud: token_data.claims.aud,
        sub: token_data.claims.sub,
        iss: token_data.claims.iss,
        custom_metadata: token_data.claims.custom_metadata,
        exp: token_data.claims.exp,
        iat: token_data.claims.iat,
        linked_accounts: match token_data.claims.linked_accounts {
            Some(accounts) => serde_json::from_str(&accounts).unwrap_or_default(),
            None => return Err(StatusCode::UNAUTHORIZED),
        },
    };

    if data.linked_accounts.is_empty() {
        dbg!("No linked accounts found");
        return Err(StatusCode::UNAUTHORIZED);
    }

    let name = data
        .linked_accounts
        .iter()
        .find(|acc| acc.account_type == "google_oauth")
        .and_then(|acc| acc.name.clone());
    let email = data
        .linked_accounts
        .iter()
        .find(|acc| acc.account_type == "google_oauth")
        .and_then(|acc| acc.email.clone());
    let solana_address = data
        .linked_accounts
        .iter()
        .find(|acc| acc.account_type == "wallet" && acc.chain_type.as_deref() == Some("solana"))
        .and_then(|acc| acc.address.clone());
    let wallet_id = data
        .linked_accounts
        .iter()
        .find(|acc| acc.account_type == "wallet" && acc.chain_type.as_deref() == Some("solana"))
        .and_then(|acc| acc.id.clone());

    let email = match email {
        Some(e) => e,
        None => return Err(StatusCode::UNAUTHORIZED),
    };

    let solana_address = match solana_address {
        Some(e) => e,
        None => return Err(StatusCode::UNAUTHORIZED),
    };

    let wallet_id = match wallet_id {
        Some(e) => e,
        None => return Err(StatusCode::UNAUTHORIZED),
    };

    let access_token = match access_token {
        Some(e) => e.to_string(),
        None => return Err(StatusCode::UNAUTHORIZED),
    };

    let name = name.unwrap_or_default();

    let auth_user = AuthUser {
        access_token,
        wallet_id,
        email,
        name,
        solana_address,
    };
    req.extensions_mut().insert(auth_user);

    Ok(next.run(req).await)
}
