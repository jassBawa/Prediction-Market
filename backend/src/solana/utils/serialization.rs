use base64;
use bincode;
use solana_sdk::hash::Hash;

pub fn serialize_transaction<T: serde::Serialize>(
    tx: &T,
    recent_blockhash: &Hash,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let serialized = bincode::serialize(tx)
        .map_err(|e| anyhow::anyhow!("Failed to serialize transaction: {}", e))?;

    #[allow(deprecated)]
    let tx_base64 = base64::encode(&serialized);

    let recent_blockhash_str = recent_blockhash.to_string();

    Ok((tx_base64, recent_blockhash_str))
}
