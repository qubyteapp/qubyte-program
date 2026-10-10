use anchor_lang::{
    prelude::*,
    solana_program::{instruction::Instruction, program::invoke_signed},
};

use crate::{constants::*, error::QubyteError};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Venue {
    Curve,
    Pool,
}

pub struct Buyer {
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub base_account: Pubkey,
    pub wsol_account: Option<Pubkey>,
}

pub fn detect(accounts: &[AccountInfo]) -> Result<Venue> {
    match accounts.last().map(|a| a.key()) {
        Some(PUMP_PROGRAM_ID) if accounts.len() == PUMP_BUY_ACCOUNTS => Ok(Venue::Curve),
        Some(PUMP_AMM_PROGRAM_ID) if accounts.len() == AMM_BUY_ACCOUNTS => Ok(Venue::Pool),
        _ => err!(QubyteError::InvalidVenue),
    }
}

pub fn volume_accumulator<'a, 'info>(
    venue: Venue,
    accounts: &'a [AccountInfo<'info>],
) -> &'a AccountInfo<'info> {
    match venue {
        Venue::Curve => &accounts[11],
        Venue::Pool => &accounts[12],
    }
}

pub fn check(venue: Venue, accounts: &[AccountInfo], buyer: &Buyer) -> Result<()> {
    match venue {
        Venue::Curve => check_curve(accounts, buyer),
        Venue::Pool => check_pool(accounts, buyer),
    }
}

fn check_curve(accounts: &[AccountInfo], buyer: &Buyer) -> Result<()> {
    require_keys_eq!(accounts[1].key(), buyer.mint, QubyteError::InvalidVenue);
    require_keys_eq!(accounts[2].key(), WSOL_MINT, QubyteError::InvalidVenue);
    require_keys_eq!(accounts[8].key(), buyer.owner, QubyteError::InvalidVenue);
    require_keys_eq!(
        accounts[9].key(),
        buyer.base_account,
        QubyteError::InvalidVenue
    );
    Ok(())
}

fn check_pool(accounts: &[AccountInfo], buyer: &Buyer) -> Result<()> {
    let mint = buyer.mint;
    let pool_authority =
        Pubkey::find_program_address(&[b"pool-authority", mint.as_ref()], &PUMP_PROGRAM_ID).0;
    let canonical_pool = Pubkey::find_program_address(
        &[
            b"pool",
            &0u16.to_le_bytes(),
            pool_authority.as_ref(),
            mint.as_ref(),
            WSOL_MINT.as_ref(),
        ],
        &PUMP_AMM_PROGRAM_ID,
    )
    .0;
    let wsol = buyer
        .wsol_account
        .ok_or(error!(QubyteError::InvalidVenue))?;
    require_keys_eq!(accounts[0].key(), canonical_pool, QubyteError::InvalidVenue);
    require_keys_eq!(accounts[1].key(), buyer.owner, QubyteError::InvalidVenue);
    require_keys_eq!(accounts[3].key(), mint, QubyteError::InvalidVenue);
    require_keys_eq!(accounts[4].key(), WSOL_MINT, QubyteError::InvalidVenue);
    require_keys_eq!(
        accounts[5].key(),
        buyer.base_account,
        QubyteError::InvalidVenue
    );
    require_keys_eq!(accounts[6].key(), wsol, QubyteError::InvalidVenue);
    Ok(())
}

pub fn buy<'info>(
    venue: Venue,
    accounts: &[AccountInfo<'info>],
    owner: &Pubkey,
    amount_in: u64,
    min_out: u64,
    signer_seeds: &[&[u8]],
) -> Result<()> {
    let (program_id, discriminator) = match venue {
        Venue::Curve => (PUMP_PROGRAM_ID, PUMP_BUY_EXACT_QUOTE_IN_V3_DISCRIMINATOR),
        Venue::Pool => (PUMP_AMM_PROGRAM_ID, AMM_BUY_EXACT_QUOTE_IN_V2_DISCRIMINATOR),
    };
    let mut data = Vec::with_capacity(24);
    data.extend_from_slice(&discriminator);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&min_out.to_le_bytes());
    let ix = Instruction {
        program_id,
        accounts: accounts
            .iter()
            .map(|a| AccountMeta {
                pubkey: a.key(),
                is_signer: a.is_signer || a.key() == *owner,
                is_writable: a.is_writable,
            })
            .collect(),
        data,
    };
    invoke_signed(&ix, accounts, &[signer_seeds])?;
    Ok(())
}
