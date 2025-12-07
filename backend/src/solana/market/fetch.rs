use crate::solana::types::Market;
use anchor_lang::AnchorDeserialize;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

pub fn fetch_market(
    rpc: &RpcClient,
    market_pubkey: &Pubkey,
) -> Result<Market, Box<dyn std::error::Error>> {
    let account = rpc.get_account(market_pubkey)?;

    let mut data = &account.data[8..];
    let market = Market::deserialize(&mut data)
        .map_err(|e| anyhow::anyhow!("Failed to deserialize market: {}", e))?;

    Ok(market)
}
