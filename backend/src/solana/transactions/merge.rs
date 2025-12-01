use anchor_client::{solana_sdk::pubkey::Pubkey, Client};
use solana_client::rpc_client::RpcClient;

use solana_sdk::{message::Message, signature::Keypair, signer::Signer, transaction::Transaction};
use std::{str::FromStr, sync::Arc};

use crate::solana::{
    accounts::get_ata_address,
    client::SolanaClient,
    market::fetch_market,
    utils::{detect_cluster, load_fee_payer, serialize_transaction},
};

pub async fn generate_merge_transaction(
    client: &SolanaClient,
    market_address: &str,
    user_wallet: &str,
    amount: u64,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let rpc = RpcClient::new(client.rpc_url());
    let program_pubkey = Pubkey::from_str(client.program_id())?;
    let market_pubkey = Pubkey::from_str(market_address)?;
    let user_pubkey = Pubkey::from_str(user_wallet)?;

    let market = fetch_market(&rpc, &market_pubkey)?;
    let market_id = market.market_id;

    // Get fee payer
    let fee_payer = load_fee_payer()?;

    let recent_blockhash = rpc
        .get_latest_blockhash()
        .map_err(|e| anyhow::anyhow!("RPC error: {}", e))?;

    // Derive all required accounts
    let collateral_mint = market.collateral_mint;
    let collateral_vault = market.collateral_vault;
    let yes_mint = market.yes_mint;
    let no_mint = market.no_mint;

    println!("market {:?}", market);

    let user_collateral = get_ata_address(&user_pubkey, &collateral_mint);
    let yes_ata = get_ata_address(&user_pubkey, &yes_mint);
    let no_ata = get_ata_address(&user_pubkey, &no_mint);

    let cluster = detect_cluster(client.rpc_url());

    // Store fee_payer pubkey before moving into Arc
    let fee_payer_pubkey = fee_payer.pubkey();
    let fee_payer_arc: Arc<Keypair> = Arc::new(fee_payer);
    let provider = Client::new_with_options(
        cluster,
        fee_payer_arc.clone(),
        anchor_client::solana_sdk::commitment_config::CommitmentConfig::confirmed(),
    );

    let program = provider.program(program_pubkey)?;

    let instruction = program
        .request()
        .accounts(predix_program::accounts::MergeToken {
            user: user_pubkey,
            market: market_pubkey,
            collateral_vault,
            user_collateral,
            yes_mint,
            no_mint,
            yes_ata,
            no_ata,
            system_program: solana_sdk::system_program::ID,
            token_program: spl_token::ID,
        })
        .args(predix_program::instruction::MergeTokens { market_id, amount })
        .instructions()?
        .pop()
        .ok_or_else(|| anyhow::anyhow!("Failed to build instruction"))?;

    let message = Message::new(&[instruction], Some(&fee_payer_pubkey));
    let mut tx = Transaction::new_unsigned(message);

    tx.try_partial_sign(&[&*fee_payer_arc], recent_blockhash)
        .map_err(|e| anyhow::anyhow!("Failed to partially sign transaction: {}", e))?;

    let (tx_base64, recent_blockhash_str) = serialize_transaction(&tx, &recent_blockhash)?;

    Ok((tx_base64, recent_blockhash_str))
}
