use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

use crate::auth::privy::validate_privy_jwt;

pub async fn auth_middleware(mut req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok());

    let Some(auth) = auth_header else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    let token = auth.trim_start_matches("Bearer ");

    match validate_privy_jwt(token).await {
        Ok(claims) => {
            req.extensions_mut().insert(claims);
            Ok(next.run(req).await)
        }
        Err(_) => Err(StatusCode::UNAUTHORIZED),
    }
}
