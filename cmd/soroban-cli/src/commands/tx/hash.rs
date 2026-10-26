use hex;
use std::ffi::OsString;

use crate::{commands::global, config::network, utils::transaction_env_hash};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    TxEnvelopeFromStdin(#[from] super::xdr::Error),
    #[error(transparent)]
    XdrToBase64(#[from] crate::xdr::Error),
    #[error(transparent)]
    Config(#[from] network::Error),
}

// Command to return the transaction hash submitted to a network
/// e.g. `stellar tx hash file.txt` or `cat file.txt | stellar tx hash`
#[derive(Debug, clap::Parser, Clone, Default)]
#[group(skip)]
pub struct Cmd {
    /// Base-64 transaction envelope XDR or file containing XDR to decode, or stdin if empty
    #[arg()]
    pub tx_xdr: Option<OsString>,

    #[clap(flatten)]
    pub network: network::Args,
}

impl Cmd {
    pub fn run(&self, global_args: &global::Args) -> Result<(), Error> {
        let tx = super::xdr::tx_envelope_from_input(&self.tx_xdr)?;
        let network = &self.network.resolve(&global_args.locator, false)?;
        println!(
            "{}",
            hex::encode(transaction_env_hash(&tx, &network.network_passphrase)?)
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        utils::{fee_bump_transaction_hash, transaction_hash, XDR_DEPTH_LIMIT},
        xdr::{
            FeeBumpTransaction, FeeBumpTransactionEnvelope, FeeBumpTransactionExt,
            FeeBumpTransactionInnerTx, Limits, Memo, MuxedAccount, Preconditions, SequenceNumber,
            Transaction, TransactionEnvelope, TransactionExt, TransactionV1Envelope, Uint256, VecM,
            WriteXdr,
        },
    };

    const TESTNET_PASSPHRASE: &str = "Test SDF Network ; September 2015";

    fn sample_v1_envelope() -> TransactionV1Envelope {
        TransactionV1Envelope {
            tx: Transaction {
                source_account: MuxedAccount::Ed25519(Uint256([1u8; 32])),
                fee: 100,
                seq_num: SequenceNumber(1),
                cond: Preconditions::None,
                memo: Memo::None,
                operations: VecM::default(),
                ext: TransactionExt::V0,
            },
            signatures: VecM::default(),
        }
    }

    #[test]
    fn hashes_v1_transaction_envelope() {
        let v1_env = sample_v1_envelope();
        let expected = transaction_hash(&v1_env.tx, TESTNET_PASSPHRASE).unwrap();
        let env = TransactionEnvelope::Tx(v1_env);
        let xdr = env.to_xdr_base64(Limits::depth(XDR_DEPTH_LIMIT)).unwrap();
        let decoded =
            crate::commands::tx::xdr::tx_envelope_from_input(&Some(OsString::from(xdr))).unwrap();
        let actual = transaction_env_hash(&decoded, TESTNET_PASSPHRASE).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn hashes_fee_bump_transaction_envelope() {
        let v1_env = sample_v1_envelope();
        let inner_hash = transaction_hash(&v1_env.tx, TESTNET_PASSPHRASE).unwrap();
        let fee_bump = FeeBumpTransaction {
            fee_source: MuxedAccount::Ed25519(Uint256([2u8; 32])),
            fee: 10_000,
            inner_tx: FeeBumpTransactionInnerTx::Tx(v1_env),
            ext: FeeBumpTransactionExt::V0,
        };
        let expected = fee_bump_transaction_hash(&fee_bump, TESTNET_PASSPHRASE).unwrap();
        let env = TransactionEnvelope::TxFeeBump(FeeBumpTransactionEnvelope {
            tx: fee_bump,
            signatures: VecM::default(),
        });
        let xdr = env.to_xdr_base64(Limits::depth(XDR_DEPTH_LIMIT)).unwrap();
        let decoded =
            crate::commands::tx::xdr::tx_envelope_from_input(&Some(OsString::from(xdr))).unwrap();
        let actual = transaction_env_hash(&decoded, TESTNET_PASSPHRASE).unwrap();
        assert_eq!(actual, expected);
        assert_ne!(actual, inner_hash);
    }
}
