use solana_sdk::pubkey::Pubkey;
use spl_token;
use std::str::FromStr;

pub fn get_ata_address(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    let associated_token_program_id =
        Pubkey::from_str("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL")
            .expect("Invalid Associated Token Program ID");

    let (ata, _) = Pubkey::find_program_address(
        &[owner.as_ref(), spl_token::id().as_ref(), mint.as_ref()],
        &associated_token_program_id,
    );
    ata
}
