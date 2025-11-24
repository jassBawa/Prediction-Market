use super::types::{AccountMeta, MatchFill};
use anchor_client::{
    solana_sdk::{
        commitment_config::CommitmentConfig,
        instruction::AccountMeta as SolanaAccountMeta,
        pubkey::Pubkey,
        signature::{read_keypair_file, Keypair, Signer},
    },
    Client, Cluster,
};
use solana_client::rpc_client::RpcClient;
use std::{str::FromStr, sync::Arc};

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

    pub fn get_latest_blockhash(
        &self,
    ) -> Result<solana_sdk::hash::Hash, Box<dyn std::error::Error>> {
        let client = RpcClient::new(self.rpc_url.clone());
        Ok(client.get_latest_blockhash()?)
    }

    pub async fn execute_match_multi(
        &self,
        market_address: &str,
        match_fills: Vec<MatchFill>,
        remaining_accounts: Vec<AccountMeta>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let admin_keypair = self.load_admin_keypair()?;
        let admin_keypair_arc: Arc<Keypair> = Arc::new(admin_keypair);
        let admin_pubkey = admin_keypair_arc.pubkey();

        let cluster = self.get_cluster();

        let provider = Client::new_with_options(
            cluster,
            admin_keypair_arc.clone(),
            CommitmentConfig::confirmed(),
        );

        let program_id = Pubkey::from_str(&self.program_id)?;
        let program = provider.program(program_id)?;

        println!(
            "  → Building instruction with {} match fills and {} accounts...",
            match_fills.len(),
            remaining_accounts.len()
        );

        let mut remaining_account_metas = Vec::new();
        for (i, account) in remaining_accounts.iter().enumerate() {
            let pubkey = Pubkey::from_str(&account.pubkey)?;
            println!(
                "  → Account[{}]: {} (writable: {}, signer: {})",
                i,
                &account.pubkey[..8],
                account.is_writable,
                account.is_signer
            );
            if account.is_writable {
                remaining_account_metas.push(SolanaAccountMeta::new(pubkey, account.is_signer));
            } else {
                remaining_account_metas
                    .push(SolanaAccountMeta::new_readonly(pubkey, account.is_signer));
            }
        }

        let market_pubkey = Pubkey::from_str(market_address)?;
        let token_program_id = spl_token::id();

        // Convert match_fills to Vec for args
        let fills_vec: Vec<predix_program::MatchFill> = match_fills
            .iter()
            .map(|f| predix_program::MatchFill {
                shares: f.shares,
                price: f.price,
                side: match f.side {
                    predix_program::TradeSide::Yes => predix_program::TradeSide::Yes,
                    predix_program::TradeSide::No => predix_program::TradeSide::No,
                },
            })
            .collect();

        println!("  → Submitting transaction...");
        let signature = program
            .request()
            .accounts(predix_program::accounts::ExecuteMatch {
                market: market_pubkey,
                admin: admin_pubkey,
                token_program: token_program_id,
            })
            .args(predix_program::instruction::ExecuteMatchMulti { fills: fills_vec })
            .accounts(remaining_account_metas)
            .send()
            .await?;
        println!("  → Transaction submitted successfully!");

        Ok(signature.to_string())
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
