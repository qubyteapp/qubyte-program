use anchor_lang::prelude::*;

#[account(zero_copy)]
pub struct Identity {
    pub prepared_pubkey: [u8; 1024],
    pub pubkey_hash: [u8; 32],
    pub nonce: u64,
    pub version: u8,
    pub bump: u8,
    pub _padding: [u8; 6],
}

#[account]
#[derive(InitSpace)]
pub struct PendingLaunch {
    pub creator: Pubkey,
    pub metadata_hash: [u8; 32],
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct CoinState {
    pub mint: Pubkey,
    pub creator: Pubkey,
    pub fee_vault: Pubkey,
    pub fee_vault_bump: u8,
    pub buyback_bps: u16,
    pub treasury_bps: u16,
    pub total_burned: u64,
    pub buyback_carry: u64,
    pub last_buyback_slot: u64,
    pub bump: u8,
    pub launched_at: i64,
    pub total_sol_spent: u64,
    pub total_sol_to_treasury: u64,
    pub buyback_count: u32,
    pub last_buyback_at: i64,
}
