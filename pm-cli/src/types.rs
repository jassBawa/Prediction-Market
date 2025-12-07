use anchor_lang::declare_program;

declare_program!(predix_program);

pub use self::predix_program::{
    client::{accounts, args},
    // accounts::{Market},
    types::{MarketOutcome, MatchFill, TradeSide},
};
