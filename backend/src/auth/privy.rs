use std::env;

use anyhow::{Context, Result};
use privy_rs::{
    generated::{types::WalletRpcResponse, ResponseValue},
    AuthorizationContext, PrivateKey, PrivyClient,
};

pub struct PClient {
    pub client: PrivyClient,
}
impl PClient {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let app_id =
            std::env::var("PRIVY_APP_ID").context("PRIVY_APP_ID environment variable not set")?;

        let app_secret = env::var("PRIVY_APP_SECRET")
            .context("PRIVY_APP_SECRET environment variable not set")?;

        let client = PrivyClient::new(app_id, app_secret)?;

        Ok(Self { client })
    }

    pub async fn sign_message(
        &self,
        wallet_address: &str,
        message: &str,
    ) -> Result<ResponseValue<WalletRpcResponse>, Box<dyn std::error::Error>> {
        let auth_key =
            env::var("PRIVY_SIGNER_PRIVATE_KEY").expect("PRIVY_AUTH_KEY environment not set");
        let ctx = AuthorizationContext::new().push(PrivateKey(auth_key.to_string()));

        let res = self
            .client
            .wallets()
            .solana()
            .sign_message(wallet_address, message, &ctx, None)
            .await?;

        Ok(res)
    }
}
