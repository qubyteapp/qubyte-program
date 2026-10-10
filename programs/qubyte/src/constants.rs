use anchor_lang::prelude::*;

#[constant]
pub const IDENTITY_VERSION: u8 = 1;

#[constant]
pub const IDENTITY_SEED: &[u8] = b"identity";

#[constant]
pub const COIN_STATE_SEED: &[u8] = b"coin";

#[constant]
pub const PENDING_LAUNCH_SEED: &[u8] = b"pending_launch";

pub const DOMAIN_LAUNCH: &[u8] = b"qubyte/launch/v2";

#[constant]
pub const LAUNCH_SERVICE_FEE_LAMPORTS: u64 = 20_000_000;

#[constant]
pub const OPENING_BURN_LAMPORTS: u64 = 10_000_000;

#[constant]
pub const BUYBACK_BPS: u16 = 8_000;
#[constant]
pub const TREASURY_BPS: u16 = 2_000;

pub const TREASURY: Pubkey = pubkey!("FQBE3jXfRz3Gk6yUqFZQUtpup96nGWGRWkmZzZucCj7M");

#[constant]
pub const FEE_VAULT_SEED: &[u8] = b"fee_vault";

pub const MAX_NAME_LEN: usize = 32;
pub const MAX_SYMBOL_LEN: usize = 13;
pub const MAX_URI_LEN: usize = 200;

pub const PUMP_PROGRAM_ID: Pubkey = pubkey!("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");
pub const PUMP_CREATE_V2_DISCRIMINATOR: [u8; 8] = [214, 144, 76, 236, 95, 139, 49, 180];
pub const PUMP_CREATE_V2_ACCOUNTS: usize = 16;

pub const PUMP_AMM_PROGRAM_ID: Pubkey = pubkey!("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA");
pub const WSOL_MINT: Pubkey = pubkey!("So11111111111111111111111111111111111111112");

pub const PUMP_BUY_EXACT_QUOTE_IN_V3_DISCRIMINATOR: [u8; 8] =
    [225, 247, 80, 30, 213, 179, 132, 136];
pub const PUMP_BUY_ACCOUNTS: usize = 17;
pub const AMM_BUY_EXACT_QUOTE_IN_V2_DISCRIMINATOR: [u8; 8] = [194, 171, 28, 70, 104, 77, 91, 47];
pub const AMM_BUY_ACCOUNTS: usize = 17;

#[constant]
pub const MIN_CLAIM_LAMPORTS: u64 = 1_000_000;
pub const VOLUME_ACCUMULATOR_MAX_LEN: usize = 200;
