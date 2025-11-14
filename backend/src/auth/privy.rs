use anyhow::{Context, Result};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};

use crate::auth::claims::PrivyClaims;

const PRIVY_ISSUER: &str = "privy.io";

pub async fn validate_privy_jwt(token: &str) -> Result<PrivyClaims> {
    let verification_key = std::env::var("PRIVY_VERIFICATION_KEY")
        .context("PRIVY_VERIFICATION_KEY environment variable not set")?;
    let app_id =
        std::env::var("PRIVY_APP_ID").context("PRIVY_APP_ID environment variable not set")?;

    let mut validation = Validation::new(Algorithm::ES256);
    validation.set_issuer(&[PRIVY_ISSUER]);
    validation.set_audience(&[app_id]);

    let decoding_key = DecodingKey::from_ec_pem(verification_key.as_bytes())
        .context("failed to parse PRIVY_VERIFICATION_KEY as PEM")?;

    let token_data = decode::<PrivyClaims>(token, &decoding_key, &validation)
        .context("failed to decode privy token")?;

    Ok(token_data.claims)
}
