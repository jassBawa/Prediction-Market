use anchor_client::solana_sdk::signature::Keypair;
use anchor_lang::{AccountDeserialize, Discriminator};
use anyhow::Result;

use serde_json::json;
use std::rc::Rc;

use crate::types::predix_program::accounts::Market;

pub async fn list_markets(program: anchor_client::Program<Rc<Keypair>>) -> Result<()> {
    println!("Fetching markets from blockchain....\n");
    let discriminator = Market::DISCRIMINATOR;
    let mut markets_json = vec![];
    let rpc_accounts = program.rpc().get_program_accounts(&program.id()).await?;

    let mut found_any = false;

    for (pubkey, account) in rpc_accounts {
        if account.data.len() < 8 || &account.data[0..8] != discriminator {
            continue;
        }
        let market: Market = AccountDeserialize::try_deserialize(&mut account.data.as_slice())?;
        found_any = true;

        markets_json.push(json!({
            "market_pda": pubkey.to_string(),
            "market_id": market.market_id,
            "authority": market.authority.to_string(),
            "metadata": market.metadata,
            "collateral_vault": market.collateral_vault.to_string(),
            "collateral_mint": market.collateral_mint.to_string(),
            "yes_mint": market.yes_mint.to_string(),
            "no_mint": market.no_mint.to_string(),
            "expiration_timestamp": market.expiration_timestamp,
            "outcome": format!("{:?}", market.outcome),
            "is_settled": market.is_settled
        }));
    }
    println!("{}", serde_json::to_string_pretty(&markets_json)?);

    if !found_any {
        println!("No market accounts found.");
    }

    Ok(())
}
