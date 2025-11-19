use std::rc::Rc;

use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use spl_token::{instruction::initialize_mint2, state::Mint};

pub async fn create_mint(
    client: &RpcClient,
    payer: &Keypair,
    mint_authority: Pubkey,
) -> Result<Pubkey> {
    let mint = Rc::new(Keypair::new());
    let space = Mint::LEN;
    let rent = client.get_minimum_balance_for_rent_exemption(space).await?;

    let create_account_ix = system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        rent,
        space as u64,
        &spl_token::id(),
    );

    let initialize_mint_ix =
        initialize_mint2(&spl_token::id(), &mint.pubkey(), &mint_authority, None, 6)?;

    let blockhash = client.get_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[create_account_ix, initialize_mint_ix],
        Some(&payer.pubkey()),
        &[payer, mint.as_ref()],
        blockhash,
    );

    client.send_and_confirm_transaction(&tx).await?;

    Ok(mint.pubkey())
}
