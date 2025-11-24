use axum::{
    body::Body,
    http::{HeaderMap, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde_json::json;

use crate::auth::claims::{AuthUser, LinkedAccount, RawClaims};

fn error_response(msg: &str, code: StatusCode) -> Response {
    (code, Json(json!({ "error": msg }))).into_response()
}

pub async fn auth_middleware(headers: HeaderMap, mut req: Request<Body>, next: Next) -> Response {
    let access_token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    let access_token = match access_token {
        Some(t) => t.to_string(),
        None => return error_response("Missing Bearer token", StatusCode::UNAUTHORIZED),
    };

    let id_token = headers.get("privy-id-token").and_then(|v| v.to_str().ok());

    let id_token = match id_token {
        Some(v) => v.to_string(),
        None => return error_response("Missing privy-id-token", StatusCode::UNAUTHORIZED),
    };

    let public_key = match std::env::var("PRIVY_VERIFICATION_PEM") {
        Ok(v) => v,
        Err(_) => {
            return error_response(
                "PRIVY_VERIFICATION_PEM missing",
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        }
    };

    let app_id = match std::env::var("PRIVY_APP_ID") {
        Ok(v) => v,
        Err(_) => return error_response("PRIVY_APP_ID missing", StatusCode::INTERNAL_SERVER_ERROR),
    };

    let mut validation = Validation::new(jsonwebtoken::Algorithm::ES256);
    validation.set_issuer(&["privy.io"]);
    validation.set_audience(&[&app_id]);

    let decoding_key = match DecodingKey::from_ec_pem(public_key.as_bytes()) {
        Ok(k) => k,
        Err(_) => return error_response("Invalid Privy PEM key", StatusCode::UNAUTHORIZED),
    };

    let token_data = match decode::<RawClaims>(&id_token, &decoding_key, &validation) {
        Ok(t) => t,
        Err(e) => {
            return error_response(
                &format!("Token verification failed: {}", e),
                StatusCode::UNAUTHORIZED,
            )
        }
    };

    let linked: Vec<LinkedAccount> = match token_data.claims.linked_accounts {
        Some(s) => serde_json::from_str(&s).unwrap_or_default(),
        None => return error_response("No linked accounts in token", StatusCode::UNAUTHORIZED),
    };

    if linked.is_empty() {
        return error_response("Linked accounts empty", StatusCode::UNAUTHORIZED);
    }

    let email = linked.iter().find_map(|a| match a.account_type.as_str() {
        "google_oauth" => a.email.clone(),
        "email" => a.address.clone(),
        "email_address" => a.address.clone(),
        _ => None,
    });

    // println!("Extracted email before match: {:?}", email);

    let email = match email {
        Some(v) => v,
        None => return error_response("Email not found", StatusCode::UNAUTHORIZED),
    };
    // println!("Final email string: {:?}", email);

    let sol_address = linked
        .iter()
        .find(|a| a.account_type == "wallet" && a.chain_type.as_deref() == Some("solana"))
        .and_then(|a| a.address.clone());

    let sol_address = match sol_address {
        Some(v) => v,
        None => return error_response("Solana wallet not found", StatusCode::UNAUTHORIZED),
    };
    // println!("soladdress: {:?}", sol_address);

    let wallet_id = linked
        .iter()
        .find(|a| a.account_type == "wallet" && a.chain_type.as_deref() == Some("solana"))
        .and_then(|a| a.address.clone());
    // println!(" wallet_id {:?}", wallet_id);

    let wallet_id = match wallet_id {
        Some(v) => v,
        None => return error_response("Solana wallet ID not found", StatusCode::UNAUTHORIZED),
    };
    // println!("walletid: {:?}", wallet_id);

    let name = linked
        .iter()
        .find(|acc| acc.account_type == "google_oauth")
        .and_then(|acc| acc.name.clone())
        .unwrap_or_default();

    // println!("name: {:?}", name);
    req.extensions_mut().insert(AuthUser {
        access_token,
        wallet_id,
        email,
        name,
        solana_address: sol_address,
    });

    next.run(req).await
}
