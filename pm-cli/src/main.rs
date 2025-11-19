use anchor_client::{Client, Cluster, solana_sdk::signature::read_keypair_file};
use anchor_lang::declare_program;
use anyhow::Result;
use clap::{Parser, Subcommand};
use pm_cli::commands;
use std::rc::Rc;

declare_program!(predix_program);

#[derive(Parser)]
#[command(name = "pm-cli")]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    CreateMarket {
        #[arg(long)]
        market_id: u64,

        #[arg(long)]
        metadata: String,

        #[arg(long)]
        end_time: i64,
    },
    ResolveMarket {
        #[arg(long)]
        market_id: u64,

        #[arg(long)]
        outcome: String
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Args::parse();
    let keypair_path = shellexpand::tilde("~/.config/solana/id.json").to_string();
    let payer = read_keypair_file(keypair_path)?;
    let client = Client::new(Cluster::Localnet, Rc::new(payer.insecure_clone()));
    let program = client.program(predix_program::ID).unwrap();

    match cli.command {
        Commands::CreateMarket {
            market_id,
            metadata,
            end_time,
        } => {
            commands::create_market(program, payer, market_id, metadata, end_time).await?;
        }
        Commands::ResolveMarket { market_id, outcome} => {
            commands::resolve_market(program, payer, market_id, outcome).await?;
        }
    }

    Ok(())
}
