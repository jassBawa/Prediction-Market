use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

pub const LAMPORTS_PER_TOKEN: f64 = 1_000_000.0;

pub fn decimal_to_lamports(decimal: Decimal) -> Result<u64, Box<dyn std::error::Error>> {
    decimal
        .to_f64()
        .ok_or_else(|| anyhow::anyhow!("Invalid decimal: cannot convert to f64").into())
        .map(|f| (f * LAMPORTS_PER_TOKEN) as u64)
}
