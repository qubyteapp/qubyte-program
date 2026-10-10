use anchor_lang::prelude::*;
use solana_falcon512::Falcon512Pubkey;
use solana_sha256_hasher::hash;

use crate::{constants::*, error::QubyteError, state::Identity};

#[derive(Accounts)]
#[instruction(falcon_pubkey: Vec<u8>, pubkey_hash: [u8; 32])]
pub struct InitIdentity<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + core::mem::size_of::<Identity>(),
        seeds = [IDENTITY_SEED, &[IDENTITY_VERSION], &pubkey_hash],
        bump
    )]
    pub identity: AccountLoader<'info, Identity>,
    pub system_program: Program<'info, System>,
}

pub fn handle_init_identity(
    ctx: Context<InitIdentity>,
    falcon_pubkey: Vec<u8>,
    pubkey_hash: [u8; 32],
) -> Result<()> {
    require!(
        hash(&falcon_pubkey).to_bytes() == pubkey_hash,
        QubyteError::PubkeyHashMismatch
    );
    let mut identity = ctx.accounts.identity.load_init()?;
    prepare_into(&falcon_pubkey, &mut identity.prepared_pubkey)?;
    identity.pubkey_hash = pubkey_hash;
    identity.nonce = 0;
    identity.version = IDENTITY_VERSION;
    identity.bump = ctx.bumps.identity;
    Ok(())
}

#[inline(never)]
fn prepare_into(falcon_pubkey: &[u8], out: &mut [u8; 1024]) -> Result<()> {
    let pubkey = Falcon512Pubkey::try_from_slice(falcon_pubkey)
        .map_err(|_| error!(QubyteError::InvalidFalconPubkey))?;
    let prepared = pubkey
        .try_prepare_pubkey()
        .map_err(|_| error!(QubyteError::InvalidFalconPubkey))?;
    *out = *prepared.as_bytes();
    Ok(())
}
