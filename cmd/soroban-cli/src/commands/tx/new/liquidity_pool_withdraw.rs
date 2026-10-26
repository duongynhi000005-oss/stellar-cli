use clap::Parser;

use super::liquidity_pool_deposit::parse_pool_id;
use crate::{commands::tx, tx::builder, xdr};

#[derive(Parser, Debug, Clone)]
#[group(skip)]
pub struct Cmd {
    #[command(flatten)]
    pub tx: tx::Args,
    #[clap(flatten)]
    pub op: Args,
}

#[derive(Debug, clap::Args, Clone)]
pub struct Args {
    /// Liquidity pool ID to withdraw from
    #[arg(long)]
    pub liquidity_pool_id: String,

    /// Amount of pool shares to withdraw, in stroops
    #[arg(long)]
    pub amount: builder::Amount,

    /// Minimum amount of the first asset to receive, in stroops
    #[arg(long)]
    pub min_amount_a: builder::Amount,

    /// Minimum amount of the second asset to receive, in stroops
    #[arg(long)]
    pub min_amount_b: builder::Amount,
}

impl TryFrom<&Cmd> for xdr::OperationBody {
    type Error = tx::args::Error;
    fn try_from(
        Cmd {
            tx: _,
            op:
                Args {
                    liquidity_pool_id,
                    amount,
                    min_amount_a,
                    min_amount_b,
                },
        }: &Cmd,
    ) -> Result<Self, Self::Error> {
        let pool_id = parse_pool_id(liquidity_pool_id)?;

        Ok(xdr::OperationBody::LiquidityPoolWithdraw(
            xdr::LiquidityPoolWithdrawOp {
                liquidity_pool_id: pool_id,
                amount: amount.into(),
                min_amount_a: min_amount_a.into(),
                min_amount_b: min_amount_b.into(),
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_withdraw_accepts_hex_and_strkey_pool_id() {
        let hex_id = "001041ac1d61419a62e0c0152e59f13aff7f8f65c2f04c1371ebbc662b31f4ab";
        let from_hex = parse_pool_id(hex_id).unwrap();
        let strkey = from_hex.to_string();
        let from_strkey = parse_pool_id(&strkey).unwrap();
        assert_eq!(from_hex, from_strkey);
    }
}
