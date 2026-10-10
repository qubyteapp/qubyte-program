pub mod auth;
pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;
pub mod venue;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("QUX1RmY5cmjf1o3dCAM8vKQ8goBxNzY3us1P3JNm62M");

#[program]
pub mod qubyte {
    use super::*;

    pub fn init_identity(
        ctx: Context<InitIdentity>,
        falcon_pubkey: Vec<u8>,
        pubkey_hash: [u8; 32],
    ) -> Result<()> {
        instructions::init_identity::handle_init_identity(ctx, falcon_pubkey, pubkey_hash)
    }

    pub fn authorize_launch(
        ctx: Context<AuthorizeLaunch>,
        metadata_hash: [u8; 32],
        valid_until_slot: u64,
        nonce: u64,
        signature: Vec<u8>,
    ) -> Result<()> {
        instructions::authorize_launch::handle_authorize_launch(
            ctx,
            metadata_hash,
            valid_until_slot,
            nonce,
            signature,
        )
    }

    pub fn launch<'info>(
        ctx: Context<'info, Launch<'info>>,
        name: String,
        symbol: String,
        uri: String,
    ) -> Result<()> {
        instructions::launch::handle_launch(ctx, name, symbol, uri)
    }

    pub fn claim_and_burn<'info>(ctx: Context<'info, ClaimAndBurn<'info>>) -> Result<()> {
        instructions::claim_and_burn::handle_claim_and_burn(ctx)
    }
}
