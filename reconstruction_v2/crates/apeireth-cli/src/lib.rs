use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "apeireth-cli", version = "0.1.0")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Starts gateway + companion daemon
    Serve,
    /// Interactive terminal chat
    Chat,
    /// Inspects SHA-256 audit log
    Audit { hash: Option<String> },
    /// Searches & inspects episodes/graph
    Memory { query: String },
    /// Health & diagnostics
    Status,
}

pub async fn run_cli(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Commands::Serve => {
            println!("Starting serve...");
        }
        Commands::Chat => {
            println!("Starting chat...");
        }
        Commands::Audit { hash } => {
            println!("Auditing {:?}", hash);
        }
        Commands::Memory { query } => {
            println!("Searching memory for {}", query);
        }
        Commands::Status => {
            println!("Status: OK");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_command_parsing() {
        let args = vec!["apeireth", "status"];
        let cli = Cli::try_parse_from(args).unwrap();
        match cli.command {
            Commands::Status => {}
            _ => panic!("Expected status command"),
        }

        let args2 = vec!["apeireth", "audit", "sha256_mock_hash"];
        let cli2 = Cli::try_parse_from(args2).unwrap();
        match cli2.command {
            Commands::Audit { hash } => assert_eq!(hash, Some("sha256_mock_hash".into())),
            _ => panic!("Expected audit command"),
        }
    }
}

