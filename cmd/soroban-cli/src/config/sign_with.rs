use crate::{
    config::UnresolvedMuxedAccount,
    print::Print,
    signer::{self, ledger::LedgerEntry, Signer, SignerKind},
    xdr::{self, TransactionEnvelope},
};

use super::{
    locator,
    network::{self, Network},
    secret,
};
use crate::commands::HEADING_SIGNING;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Network(#[from] network::Error),
    #[error(transparent)]
    Signer(#[from] signer::Error),
    #[error(transparent)]
    Secret(#[from] secret::Error),
    #[error(transparent)]
    Locator(#[from] locator::Error),
    #[error(transparent)]
    Rpc(#[from] soroban_rpc::Error),
    #[error("No sign with key provided")]
    NoSignWithKey,
    #[error(transparent)]
    StrKey(#[from] stellar_strkey::DecodeError),
    #[error(transparent)]
    Xdr(#[from] xdr::Error),
    #[error(transparent)]
    Ledger(#[from] signer::ledger::Error),
    #[error("signature expiration ledger offset must be between 1 and {max} ledgers (got {value})")]
    InvalidSignatureExpirationLedgers { value: u32, max: u32 },
    #[error("computing signature expiration ledger overflowed u32 (latest_ledger={latest_ledger}, offset={offset})")]
    SignatureExpirationLedgerOverflow { latest_ledger: u32, offset: u32 },
}

/// Default number of ledgers (~5 minutes at 5s/ledger) added to `latest_ledger` for Soroban auth entry expiration.
pub const DEFAULT_SIGNATURE_EXPIRATION_LEDGERS: u32 = 60;

/// Maximum bounded relative expiration window (~24 hours at 5s/ledger) when deriving expiration from `getLatestLedger`.
pub const MAX_SIGNATURE_EXPIRATION_LEDGERS: u32 = 17_280;

#[derive(Debug, clap::Args, Clone, Default)]
#[group(skip)]
pub struct Args {
    /// Sign with a local key or key saved in OS secure storage. Can be an identity (--sign-with-key alice), a secret key (--sign-with-key SC36…), or a seed phrase (--sign-with-key "kite urban…"). If using seed phrase, `--hd-path` defaults to the `0` path.
    #[arg(
        long,
        env = "STELLAR_SIGN_WITH_KEY",
        hide_env_values = true,
        help_heading = HEADING_SIGNING
    )]
    pub sign_with_key: Option<String>,

    #[arg(long, conflicts_with = "sign_with_lab", help_heading = HEADING_SIGNING)]
    /// If using a seed phrase to sign, sets which hierarchical deterministic path to use, e.g. `m/44'/148'/{hd_path}`. Example: `--hd-path 1`. Default: `0`
    pub hd_path: Option<u32>,

    #[allow(clippy::doc_markdown)]
    /// Sign with https://lab.stellar.org
    #[arg(
        long,
        conflicts_with = "sign_with_key",
        env = "STELLAR_SIGN_WITH_LAB",
        help_heading = HEADING_SIGNING
    )]
    pub sign_with_lab: bool,

    /// Sign with a ledger wallet
    #[arg(
        long,
        conflicts_with = "sign_with_key",
        conflicts_with = "sign_with_lab",
        env = "STELLAR_SIGN_WITH_LEDGER",
        help_heading = HEADING_SIGNING
    )]
    pub sign_with_ledger: bool,

    /// Sign without prompting for approval. Only applies to signatures that require user approval, like non-root Soroban auth entries.
    #[arg(long, help_heading = HEADING_SIGNING)]
    pub auto_sign: bool,

    /// Absolute ledger sequence number at which Soroban authorization signatures expire. When set, bypasses querying `getLatestLedger` from RPC.
    #[arg(
        long,
        env = "STELLAR_SIGNATURE_EXPIRATION_LEDGER",
        help_heading = HEADING_SIGNING
    )]
    pub signature_expiration_ledger: Option<u32>,

    /// Relative number of ledgers from `getLatestLedger` after which Soroban authorization signatures expire (default: 60, max: 17280).
    #[arg(
        long,
        conflicts_with = "signature_expiration_ledger",
        env = "STELLAR_SIGNATURE_EXPIRATION_LEDGERS",
        help_heading = HEADING_SIGNING
    )]
    pub signature_expiration_ledgers: Option<u32>,
}

impl Args {
    // when a default_signer_account is provided, it will be used as the tx signer if the user does not specify a signer. The default signer should be the tx's source_account.
    pub async fn sign_tx_env(
        &self,
        tx: &TransactionEnvelope,
        locator: &locator::Args,
        network: &Network,
        quiet: bool,
        default_signer_account: Option<&UnresolvedMuxedAccount>,
    ) -> Result<TransactionEnvelope, Error> {
        let print = Print::new(quiet);
        let signer = if self.sign_with_lab {
            Signer {
                kind: SignerKind::Lab,
                print,
            }
        } else if self.sign_with_ledger {
            Signer {
                kind: SignerKind::Ledger(LedgerEntry {
                    hd_path: self.hd_path.unwrap_or_default(),
                    public_key: None,
                }),
                print,
            }
        } else {
            // default to using the source account local key, if the user did not pass in a key
            let key_or_name = match self.sign_with_key.as_deref() {
                Some(k) => k,
                None => match default_signer_account {
                    Some(UnresolvedMuxedAccount::AliasOrSecret(ref s)) => s.as_str(),
                    _ => return Err(Error::NoSignWithKey),
                },
            };

            let secret = locator.get_secret_key_with_hd_path(key_or_name, self.hd_path)?;
            secret.signer(self.hd_path, print)?
        };
        Ok(signer.sign_tx_env(tx, network).await?)
    }
    /// Validates and returns the relative ledger offset when deriving `signature_expiration_ledger` from `getLatestLedger`.
    pub fn validated_expiration_offset(&self) -> Result<u32, Error> {
        let offset = self
            .signature_expiration_ledgers
            .unwrap_or(DEFAULT_SIGNATURE_EXPIRATION_LEDGERS);
        if offset == 0 || offset > MAX_SIGNATURE_EXPIRATION_LEDGERS {
            return Err(Error::InvalidSignatureExpirationLedgers {
                value: offset,
                max: MAX_SIGNATURE_EXPIRATION_LEDGERS,
            });
        }
        Ok(offset)
    }

    /// Computes a bounded `signature_expiration_ledger` from a reported `latest_ledger`.
    pub fn compute_expiration_ledger(&self, latest_ledger: u32) -> Result<u32, Error> {
        if let Some(explicit) = self.signature_expiration_ledger {
            return Ok(explicit);
        }
        let offset = self.validated_expiration_offset()?;
        latest_ledger
            .checked_add(offset)
            .ok_or(Error::SignatureExpirationLedgerOverflow {
                latest_ledger,
                offset,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_signature_expiration_ledger_bypasses_offset() {
        let args = Args {
            signature_expiration_ledger: Some(500_000),
            ..Default::default()
        };
        assert_eq!(args.compute_expiration_ledger(u32::MAX).unwrap(), 500_000);
    }

    #[test]
    fn default_signature_expiration_offset_adds_60_ledgers() {
        let args = Args::default();
        assert_eq!(args.validated_expiration_offset().unwrap(), 60);
        assert_eq!(args.compute_expiration_ledger(1_000).unwrap(), 1_060);
    }

    #[test]
    fn custom_signature_expiration_offset_respects_bounds() {
        let valid = Args {
            signature_expiration_ledgers: Some(MAX_SIGNATURE_EXPIRATION_LEDGERS),
            ..Default::default()
        };
        assert_eq!(
            valid.compute_expiration_ledger(100).unwrap(),
            100 + MAX_SIGNATURE_EXPIRATION_LEDGERS
        );

        let zero = Args {
            signature_expiration_ledgers: Some(0),
            ..Default::default()
        };
        assert!(matches!(
            zero.validated_expiration_offset(),
            Err(Error::InvalidSignatureExpirationLedgers { value: 0, .. })
        ));

        let too_large = Args {
            signature_expiration_ledgers: Some(MAX_SIGNATURE_EXPIRATION_LEDGERS + 1),
            ..Default::default()
        };
        assert!(matches!(
            too_large.validated_expiration_offset(),
            Err(Error::InvalidSignatureExpirationLedgers { .. })
        ));
    }

    #[test]
    fn compute_expiration_ledger_rejects_u32_overflow() {
        let args = Args::default();
        assert!(matches!(
            args.compute_expiration_ledger(u32::MAX - 10),
            Err(Error::SignatureExpirationLedgerOverflow { .. })
        ));
    }
}
