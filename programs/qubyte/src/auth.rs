use anchor_lang::prelude::*;
use solana_falcon512::{Falcon512PreparedPubkey, Falcon512Signature};
use solana_sha256_hasher::hashv;

use crate::{constants::*, error::QubyteError, state::Identity};

pub fn metadata_hash(name: &str, symbol: &str, uri: &str) -> [u8; 32] {
    hashv(&[
        &(name.len() as u32).to_le_bytes(),
        name.as_bytes(),
        &(symbol.len() as u32).to_le_bytes(),
        symbol.as_bytes(),
        &(uri.len() as u32).to_le_bytes(),
        uri.as_bytes(),
    ])
    .to_bytes()
}

pub fn launch_message(
    program_id: &Pubkey,
    creator: &Pubkey,
    mint: &Pubkey,
    metadata_hash: &[u8; 32],
    valid_until_slot: u64,
    nonce: u64,
) -> [u8; 32] {
    hashv(&[
        DOMAIN_LAUNCH,
        program_id.as_ref(),
        creator.as_ref(),
        mint.as_ref(),
        metadata_hash,
        &valid_until_slot.to_le_bytes(),
        &nonce.to_le_bytes(),
    ])
    .to_bytes()
}

pub fn verify_prepared(
    prepared: &Falcon512PreparedPubkey,
    message: &[u8; 32],
    signature: &[u8],
) -> Result<()> {
    let signature = Falcon512Signature::try_from_slice(signature)
        .map_err(|_| error!(QubyteError::InvalidSignatureLength))?;
    require!(
        signature.verify_with_prepared(message, prepared),
        QubyteError::InvalidSignature
    );
    Ok(())
}

pub fn authorize(
    identity: &mut Identity,
    message: &[u8; 32],
    signature: &[u8],
    nonce: u64,
    valid_until_slot: u64,
) -> Result<()> {
    require_gte!(
        valid_until_slot,
        Clock::get()?.slot,
        QubyteError::SignatureExpired
    );
    require_eq!(nonce, identity.nonce, QubyteError::InvalidNonce);
    let prepared = Falcon512PreparedPubkey::try_from_slice(&identity.prepared_pubkey)
        .map_err(|_| error!(QubyteError::InvalidFalconPubkey))?;
    verify_prepared(prepared, message, signature)?;
    identity.nonce = identity
        .nonce
        .checked_add(1)
        .ok_or(error!(QubyteError::InvalidNonce))?;
    Ok(())
}
