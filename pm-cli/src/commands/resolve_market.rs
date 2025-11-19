use anchor_client::solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use anyhow::Result;
use std::rc::Rc;

pub async fn resolve_market(
    program: anchor_client::Program<Rc<Keypair>>,
    payer_keypair: Keypair,
    market_id: u64,
    outcome: String,
) -> Result<()> {
    println!("Resolving market with ID: {}", market_id);

    let market_outcome = match outcome.to_lowercase().as_str() {
        "yes" => predix_program::state::market::MarketOutcome::Yes,
        "no" => predix_program::state::market::MarketOutcome::No,
        "undecided" => predix_program::state::market::MarketOutcome::Undecided,
        _ => {
            return Err(anyhow::anyhow!(
                "Invalid outcome. Must be 'yes', 'no', or 'undecided'"
            ));
        }
    };

    let (market_pda, _market_bump) = Pubkey::find_program_address(
        &[b"market", &market_id.to_le_bytes()],
        &predix_program::id(),
    );

    println!("Market PDA: {}", market_pda);
    println!("Setting outcome to: {}", outcome);

    let set_winner_tx = program
        .request()
        .accounts(predix_program::accounts::SetWinner {
            market: market_pda,
            admin: payer_keypair.pubkey(),
        })
        .args(predix_program::instruction::SetWinner {
            outcome: market_outcome,
            result: true,
        })
        .send()
        .await?;

    println!("\nMarket resolved successfully!");
    println!("Transaction signature: {}", set_winner_tx);
    println!("Market address: {}", market_pda);
    println!("Final outcome: {}", outcome);

    Ok(())
}
