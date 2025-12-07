use matching_engine::ShareType;
use solana_sdk::pubkey::Pubkey;

pub fn derive_share_mints(program_pubkey: &Pubkey, market_id: u64) -> (Pubkey, Pubkey) {
    let (yes_mint, _) =
        Pubkey::find_program_address(&[b"yes_mint", &market_id.to_le_bytes()], program_pubkey);

    let (no_mint, _) =
        Pubkey::find_program_address(&[b"no_mint", &market_id.to_le_bytes()], program_pubkey);

    (yes_mint, no_mint)
}

pub fn derive_market_pda(program_pubkey: &Pubkey, market_id: u64) -> (Pubkey, u8) {
    let market_id_bytes = market_id.to_le_bytes();
    let seeds = &[b"market", market_id_bytes.as_ref()];
    Pubkey::find_program_address(seeds, program_pubkey)
}

pub fn derive_share_mint(program_pubkey: &Pubkey, market_id: u64, share_type: ShareType) -> Pubkey {
    let (mint, _) = match share_type {
        ShareType::Yes => {
            Pubkey::find_program_address(&[b"yes_mint", &market_id.to_le_bytes()], program_pubkey)
        }
        ShareType::No => {
            Pubkey::find_program_address(&[b"no_mint", &market_id.to_le_bytes()], program_pubkey)
        }
    };
    mint
}
