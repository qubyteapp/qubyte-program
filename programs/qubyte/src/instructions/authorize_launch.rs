use anchor_lang::prelude::*;

use crate::{
    auth,
    constants::*,
    state::{Identity, PendingLaunch},
};

#[derive(Accounts)]
pub struct AuthorizeLaunch<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut)]
    pub identity: AccountLoader<'info, Identity>,
    /// CHECK: bound by the Falcon signature; the mint keypair signs `launch`.
    pub mint: UncheckedAccount<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + PendingLaunch::INIT_SPACE,
        seeds = [PENDING_LAUNCH_SEED, mint.key().as_ref()],
        bump
    )]
    pub pending_launch: Account<'info, PendingLaunch>,
    pub system_program: Program<'info, System>,
}

pub fn handle_authorize_launch(
    ctx: Context<AuthorizeLaunch>,
    metadata_hash: [u8; 32],
    valid_until_slot: u64,
    nonce: u64,
    signature: Vec<u8>,
) -> Result<()> {
    let creator = ctx.accounts.identity.key();
    let mut identity = ctx.accounts.identity.load_mut()?;

    let message = auth::launch_message(
        &crate::ID,
        &creator,
        &ctx.accounts.mint.key(),
        &metadata_hash,
        valid_until_slot,
        nonce,
    );
    auth::authorize(&mut identity, &message, &signature, nonce, valid_until_slot)?;

    ctx.accounts.pending_launch.set_inner(PendingLaunch {
        creator,
        metadata_hash,
        bump: ctx.bumps.pending_launch,
    });
    Ok(())
}
