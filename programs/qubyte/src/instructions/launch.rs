use anchor_lang::{
    prelude::*,
    solana_program::{instruction::Instruction, program::invoke},
    system_program,
};

use crate::{
    auth,
    constants::*,
    error::QubyteError,
    state::{CoinState, PendingLaunch},
};

#[derive(Accounts)]
pub struct Launch<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut)]
    pub mint: Signer<'info>,
    #[account(
        mut,
        close = payer,
        seeds = [PENDING_LAUNCH_SEED, mint.key().as_ref()],
        bump = pending_launch.bump,
    )]
    pub pending_launch: Account<'info, PendingLaunch>,
    #[account(
        mut,
        seeds = [FEE_VAULT_SEED, mint.key().as_ref()],
        bump
    )]
    pub fee_vault: SystemAccount<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + CoinState::INIT_SPACE,
        seeds = [COIN_STATE_SEED, mint.key().as_ref()],
        bump
    )]
    pub coin_state: Account<'info, CoinState>,
    #[account(mut, address = TREASURY)]
    pub treasury: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}

pub fn handle_launch<'info>(
    ctx: Context<'info, Launch<'info>>,
    name: String,
    symbol: String,
    uri: String,
) -> Result<()> {
    require!(
        name.len() <= MAX_NAME_LEN && symbol.len() <= MAX_SYMBOL_LEN && uri.len() <= MAX_URI_LEN,
        QubyteError::MetadataTooLong
    );
    require!(
        auth::metadata_hash(&name, &symbol, &uri) == ctx.accounts.pending_launch.metadata_hash,
        QubyteError::MetadataMismatch
    );

    let pay = |to: AccountInfo<'info>, lamports: u64| {
        system_program::transfer(
            CpiContext::new(
                system_program::ID,
                system_program::Transfer {
                    from: ctx.accounts.payer.to_account_info(),
                    to,
                },
            ),
            lamports,
        )
    };
    pay(
        ctx.accounts.treasury.to_account_info(),
        LAUNCH_SERVICE_FEE_LAMPORTS,
    )?;
    let fee_vault_rent = Rent::get()?
        .minimum_balance(0)
        .saturating_sub(ctx.accounts.fee_vault.lamports());
    pay(
        ctx.accounts.fee_vault.to_account_info(),
        fee_vault_rent + OPENING_BURN_LAMPORTS,
    )?;

    create_pump_coin(&ctx, &name, &symbol, &uri)?;

    ctx.accounts.coin_state.set_inner(CoinState {
        mint: ctx.accounts.mint.key(),
        creator: ctx.accounts.pending_launch.creator,
        fee_vault: ctx.accounts.fee_vault.key(),
        fee_vault_bump: ctx.bumps.fee_vault,
        buyback_bps: BUYBACK_BPS,
        treasury_bps: TREASURY_BPS,
        total_burned: 0,
        buyback_carry: OPENING_BURN_LAMPORTS,
        last_buyback_slot: 0,
        bump: ctx.bumps.coin_state,
        launched_at: Clock::get()?.unix_timestamp,
        total_sol_spent: 0,
        total_sol_to_treasury: 0,
        buyback_count: 0,
        last_buyback_at: 0,
    });
    Ok(())
}

fn create_pump_coin<'info>(
    ctx: &Context<'info, Launch<'info>>,
    name: &str,
    symbol: &str,
    uri: &str,
) -> Result<()> {
    let pump_accounts = ctx.remaining_accounts;
    require_eq!(
        pump_accounts.len(),
        PUMP_CREATE_V2_ACCOUNTS,
        ErrorCode::AccountNotEnoughKeys
    );
    require_keys_eq!(
        pump_accounts[0].key(),
        ctx.accounts.mint.key(),
        QubyteError::PumpAccountMismatch
    );
    require_keys_eq!(
        pump_accounts[5].key(),
        ctx.accounts.payer.key(),
        QubyteError::PumpAccountMismatch
    );
    require_keys_eq!(
        pump_accounts[15].key(),
        PUMP_PROGRAM_ID,
        QubyteError::PumpAccountMismatch
    );

    let mut data = Vec::with_capacity(8 + 12 + name.len() + symbol.len() + uri.len() + 32 + 1);
    data.extend_from_slice(&PUMP_CREATE_V2_DISCRIMINATOR);
    (name, symbol, uri, ctx.accounts.fee_vault.key(), false).serialize(&mut data)?;

    let ix = Instruction {
        program_id: PUMP_PROGRAM_ID,
        accounts: pump_accounts
            .iter()
            .map(|a| AccountMeta {
                pubkey: a.key(),
                is_signer: a.is_signer,
                is_writable: a.is_writable,
            })
            .collect(),
        data,
    };
    invoke(&ix, pump_accounts)?;
    Ok(())
}
