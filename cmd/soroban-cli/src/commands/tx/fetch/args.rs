use crate::color::{green, red};
use soroban_rpc::GetTransactionResponse;

use crate::{
    commands::global,
    config::network,
    rpc,
    xdr::{self, Hash},
};

#[derive(Debug, Clone, clap::Parser)]
pub struct Args {
    /// Transaction hash to fetch
    #[arg(long)]
    pub hash: Hash,

    #[command(flatten)]
    pub network: network::Args,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Network(#[from] network::Error),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error(transparent)]
    Rpc(#[from] rpc::Error),
    #[error(transparent)]
    Xdr(#[from] xdr::Error),
    #[error("transaction {tx_hash} not found on {network} network")]
    NotFound { tx_hash: Hash, network: String },
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, clap::ValueEnum, Default)]
pub enum OutputFormat {
    /// JSON output with parsed XDRs (one line, not formatted)
    #[default]
    Json,
    /// Formatted (multiline) JSON output with parsed XDRs
    JsonFormatted,
    /// Original RPC output (containing XDRs)
    Xdr,
}

impl Args {
    pub async fn fetch_transaction(
        &self,
        global_args: &global::Args,
    ) -> Result<GetTransactionResponse, Error> {
        let network = self.network.get(&global_args.locator)?;
        let client = network.rpc_client()?;
        let tx_hash = self.hash.clone();
        let tx = client.get_transaction(&tx_hash).await?;
        if tx.status == "NOT_FOUND" {
            return Err(Error::NotFound {
                tx_hash,
                network: self.network_label(&network),
            });
        }
        Ok(tx)
    }

    fn network_label(&self, resolved: &network::Network) -> String {
        if let Some(n) = &self.network.network {
            return n.clone();
        }
        for (name, (_, passphrase)) in &network::DEFAULTS {
            if *passphrase == resolved.network_passphrase {
                return (*name).to_string();
            }
        }
        crate::utils::url::redact_url(&resolved.rpc_url)
    }

    pub fn print_tx_summary(tx: &GetTransactionResponse) {
        let status = if tx.status == "SUCCESS" {
            green(&tx.status)
        } else {
            red(&tx.status)
        };
        println!("Transaction Status: {status}");
        if let Some(ledger) = tx.ledger {
            println!("Transaction Ledger: {ledger}");
        }
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_args(network: Option<&str>, rpc_url: Option<&str>, passphrase: Option<&str>) -> Args {
        Args {
            hash: Hash([0u8; 32]),
            network: network::Args {
                rpc_url: rpc_url.map(str::to_string),
                rpc_headers: Vec::new(),
                network_passphrase: passphrase.map(str::to_string),
                network: network.map(str::to_string),
            },
        }
    }

    #[test]
    fn network_label_uses_explicit_network_name_when_present() {
        let args = dummy_args(Some("testnet"), None, None);
        let resolved = network::Network {
            rpc_url: "https://soroban-testnet.stellar.org".to_string(),
            rpc_headers: Vec::new(),
            network_passphrase: network::passphrase::TESTNET.to_string(),
        };
        assert_eq!(args.network_label(&resolved), "testnet");
    }

    #[test]
    fn network_label_infers_known_network_from_passphrase_when_rpc_url_used() {
        let args = dummy_args(
            None,
            Some("https://soroban-testnet.stellar.org"),
            Some(network::passphrase::TESTNET),
        );
        let resolved = network::Network {
            rpc_url: "https://soroban-testnet.stellar.org".to_string(),
            rpc_headers: Vec::new(),
            network_passphrase: network::passphrase::TESTNET.to_string(),
        };
        assert_eq!(args.network_label(&resolved), "testnet");
    }

    #[test]
    fn network_label_falls_back_to_redacted_rpc_url_for_custom_passphrase() {
        let args = dummy_args(
            None,
            Some("https://user:secret@rpc.example.com/rpc"),
            Some("Custom Private Network ; 2026"),
        );
        let resolved = network::Network {
            rpc_url: "https://user:secret@rpc.example.com/rpc".to_string(),
            rpc_headers: Vec::new(),
            network_passphrase: "Custom Private Network ; 2026".to_string(),
        };
        assert_eq!(
            args.network_label(&resolved),
            crate::utils::url::redact_url("https://user:secret@rpc.example.com/rpc")
        );
    }
}
