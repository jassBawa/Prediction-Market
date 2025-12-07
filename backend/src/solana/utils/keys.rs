use solana_sdk::signature::Keypair;

pub fn load_fee_payer() -> Result<Keypair, Box<dyn std::error::Error>> {
    let payer_private_key = std::env::var("FEE_PAYER_PRIVATE_KEY")
        .map_err(|e| anyhow::anyhow!("FEE_PAYER_PRIVATE_KEY not set: {}", e))?;

    let pair = Keypair::from_base58_string(&payer_private_key);
    Ok(pair)
}
