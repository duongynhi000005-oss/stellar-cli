use clap::Parser;

use super::revoke_sponsorship::parse_liquidity_pool_id;
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
    /// Liquidity pool ID to deposit to
    #[arg(long)]
    pub liquidity_pool_id: String,

    /// Maximum amount of the first asset to deposit, in stroops
    #[arg(long)]
    pub max_amount_a: builder::Amount,

    /// Maximum amount of the second asset to deposit, in stroops
    #[arg(long)]
    pub max_amount_b: builder::Amount,

    /// Minimum price for the first asset in terms of the second asset as "numerator:denominator" (e.g., "1:2" means 0.5)
    #[arg(long, default_value = "1:1")]
    pub min_price: String,

    /// Maximum price for the first asset in terms of the second asset as "numerator:denominator" (e.g., "1:2" means 0.5)
    #[arg(long, default_value = "1:1")]
    pub max_price: String,
}

fn parse_price(price: &str) -> Result<xdr::Price, tx::args::Error> {
    let price_parts: Vec<&str> = price.split(':').collect();
    if price_parts.len() != 2 {
        return Err(tx::args::Error::InvalidPrice(price.to_string()));
    }

    let n: i32 = price_parts[0]
        .parse()
        .map_err(|_| tx::args::Error::InvalidPrice(price.to_string()))?;
    let d: i32 = price_parts[1]
        .parse()
        .map_err(|_| tx::args::Error::InvalidPrice(price.to_string()))?;

    if d == 0 {
        return Err(tx::args::Error::InvalidPrice(
            "denominator cannot be zero".to_string(),
        ));
    }

    Ok(xdr::Price { n, d })
}

impl TryFrom<&Cmd> for xdr::OperationBody {
    type Error = tx::args::Error;
    fn try_from(
        Cmd {
            tx: _,
            op:
                Args {
                    liquidity_pool_id,
                    max_amount_a,
                    max_amount_b,
                    min_price,
                    max_price,
                },
        }: &Cmd,
    ) -> Result<Self, Self::Error> {
        let pool_id = parse_pool_id(liquidity_pool_id)?;

        let min_price_parsed = parse_price(min_price)?;
        let max_price_parsed = parse_price(max_price)?;

        Ok(xdr::OperationBody::LiquidityPoolDeposit(
            xdr::LiquidityPoolDepositOp {
                liquidity_pool_id: pool_id,
                max_amount_a: max_amount_a.into(),
                max_amount_b: max_amount_b.into(),
                min_price: min_price_parsed,
                max_price: max_price_parsed,
            },
        ))
    }
}

pub(super) fn parse_pool_id(liquidity_pool_id: &str) -> Result<xdr::PoolId, tx::args::Error> {
    let pool_id_bytes = parse_liquidity_pool_id(liquidity_pool_id)
        .map_err(|_| tx::args::Error::InvalidPoolId(liquidity_pool_id.to_string()))?;
    let pool_id_array: [u8; 32] = pool_id_bytes
        .try_into()
        .map_err(|_| tx::args::Error::InvalidPoolId(liquidity_pool_id.to_string()))?;
    Ok(xdr::PoolId(xdr::Hash(pool_id_array)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pool_id_hex_64() {
        let hex_id = "001041ac1d61419a62e0c0152e59f13aff7f8f65c2f04c1371ebbc662b31f4ab";
        let pool_id = parse_pool_id(hex_id).unwrap();
        assert_eq!(hex::encode(pool_id.0 .0), hex_id);
    }

    #[test]
    fn test_parse_pool_id_api_prefix_72() {
        let hex_id = "001041ac1d61419a62e0c0152e59f13aff7f8f65c2f04c1371ebbc662b31f4ab";
        let api_id = format!("00000000{hex_id}");
        let pool_id = parse_pool_id(&api_id).unwrap();
        assert_eq!(hex::encode(pool_id.0 .0), hex_id);
    }

    #[test]
    fn test_parse_pool_id_strkey() {
        let hex_id = "001041ac1d61419a62e0c0152e59f13aff7f8f65c2f04c1371ebbc662b31f4ab";
        let bytes: [u8; 32] = hex::decode(hex_id).unwrap().try_into().unwrap();
        let strkey =
            stellar_strkey::Strkey::LiquidityPool(stellar_strkey::LiquidityPool(bytes)).to_string();
        let pool_id = parse_pool_id(&strkey).unwrap();
        assert_eq!(pool_id.0 .0, bytes);
    }

    #[test]
    fn test_parse_pool_id_invalid() {
        let err = parse_pool_id("invalid_pool_id").unwrap_err();
        assert!(matches!(err, tx::args::Error::InvalidPoolId(_)));
    }
}
