use anchor_client::{
    solana_sdk::{
        commitment_config::CommitmentConfig,
        pubkey::Pubkey,
        signature::{read_keypair_file, Keypair, Signer},
    },
    Client, Cluster,
};
use anchor_lang::prelude::AccountMeta;
use std::{str::FromStr, sync::Arc};

use crate::solana::types::{accounts, args, MatchFill};

#[derive(Clone)]
pub struct SolanaClient {
    rpc_url: String,
    program_id: String,
    admin_keypair_path: String,
}

impl SolanaClient {
    pub fn rpc_url(&self) -> &str {
        &self.rpc_url
    }

    pub fn program_id(&self) -> &str {
        &self.program_id
    }

    pub fn new(rpc_url: String, program_id: String, admin_keypair_path: String) -> Self {
        Self {
            rpc_url,
            program_id,
            admin_keypair_path,
        }
    }

    fn get_cluster(&self) -> Cluster {
        if self.rpc_url.contains("localhost") || self.rpc_url.contains("127.0.0.1") {
            Cluster::Localnet
        } else if self.rpc_url.contains("devnet") {
            Cluster::Devnet
        } else if self.rpc_url.contains("mainnet") {
            Cluster::Mainnet
        } else {
            Cluster::Custom(self.rpc_url.clone(), self.rpc_url.clone())
        }
    }

    pub async fn execute_match_multi(
        &self,
        market_address: &str,
        match_fills: Vec<MatchFill>,
        remaining_accounts: Vec<AccountMeta>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let admin_keypair = self.load_admin_keypair()?;
        let admin_keypair_arc = Arc::new(admin_keypair);
        let admin_pubkey = admin_keypair_arc.pubkey();

        let cluster = self.get_cluster();
        let provider = Client::new_with_options(
            cluster,
            admin_keypair_arc.clone(),
            CommitmentConfig::confirmed(),
        );

        let program_id = Pubkey::from_str(&self.program_id)?;
        let program = provider.program(program_id)?;
        let market_pubkey = Pubkey::from_str(market_address)?;

        println!(
            "Building instruction with {} match fills and {} accounts...",
            match_fills.len(),
            remaining_accounts.len()
        );
        println!("Admin wallet: {}", admin_pubkey);

        // Build main accounts
        let accounts = accounts::ExecuteMatchMulti {
            market: market_pubkey,
            admin: admin_pubkey,
            token_program: spl_token::ID,
        };

        let args = args::ExecuteMatchMulti { fills: match_fills };

        println!(
            "args accounts id marketid-> {} admin -> {}",
            market_pubkey, admin_pubkey
        );

        // Send using program.request()
        let signature = program
            .request()
            .accounts(accounts)
            .args(args)
            .accounts(remaining_accounts)
            .send()
            .await?;

        println!("Transaction successful: {}", signature);
        println!(
            "View on Solscan: https://solscan.io/tx/{}?cluster=devnet",
            signature
        );

        Ok(signature.to_string())
    }

    fn load_admin_keypair(&self) -> Result<Keypair, Box<dyn std::error::Error>> {
        read_keypair_file(&self.admin_keypair_path).map_err(|e| {
            format!(
                "Failed to load keypair from {}: {}",
                self.admin_keypair_path, e
            )
            .into()
        })
    }
}

#[derive(Debug, Clone)]
pub struct SplitTokenAccounts {
    pub market: String,
    pub user_collateral: String,
    pub collateral_vault: String,
    pub yes_mint: String,
    pub no_mint: String,
    pub yes_ata: String,
    pub no_ata: String,
    pub user: String,
}
