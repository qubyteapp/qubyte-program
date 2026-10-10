use anchor_lang::{prelude::*, system_program};
use anchor_spl::{
    token::{self, Token},
    token_2022::Token2022,
    token_interface::{self, Mint, TokenAccount},
};

use crate::{
    constants::*,
    error::QubyteError,
    state::CoinState,
    venue::{self, Buyer, Venue},
};

#[derive(Accounts)]
pub struct ClaimAndBurn<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        mut,
        seeds = [COIN_STATE_SEED, mint.key().as_ref()],
        bump = coin_state.bump,
        has_one = mint,
        has_one = fee_vault,
    )]
    pub coin_state: Box<Account<'info, CoinState>>,
    #[account(mut)]
    pub fee_vault: SystemAccount<'info>,
    #[account(mut, mint::token_program = token_2022_program)]
    pub mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        mut,
        token::mint = mint,
        token::authority = fee_vault,
        token::token_program = token_2022_program,
    )]
    pub fee_vault_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        token::mint = WSOL_MINT,
        token::authority = fee_vault,
        token::token_program = token_program,
    )]
    pub fee_vault_wsol_account: Option<Box<InterfaceAccount<'info, TokenAccount>>>,
    #[account(mut, address = TREASURY)]
    pub treasury: SystemAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub token_2022_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

#[event]
pub struct BuybackEvent {
    pub mint: Pubkey,
    pub sol_to_treasury: u64,
    pub sol_spent: u64,
    pub tokens_burned: u64,
    pub total_burned: u64,
}

pub fn handle_claim_and_burn<'info>(ctx: Context<'info, ClaimAndBurn<'info>>) -> Result<()> {
    let venue_accounts = ctx.remaining_accounts;
    let venue = venue::detect(venue_accounts)?;
    venue::check(
        venue,
        venue_accounts,
        &Buyer {
            owner: ctx.accounts.fee_vault.key(),
            mint: ctx.accounts.mint.key(),
            base_account: ctx.accounts.fee_vault_token_account.key(),
            wsol_account: ctx
                .accounts
                .fee_vault_wsol_account
                .as_ref()
                .map(|a| a.key()),
        },
    )?;
    let volume_accumulator = venue::volume_accumulator(venue, venue_accounts);

    let fee_vault = &ctx.accounts.fee_vault;
    let rent = Rent::get()?;
    let coin = &ctx.accounts.coin_state;
    let carry = coin.buyback_carry;
    let balance_before = fee_vault.lamports();
    let claimed = balance_before
        .saturating_sub(rent.minimum_balance(0))
        .saturating_sub(carry);
    let to_treasury = (claimed as u128 * coin.treasury_bps as u128 / 10_000) as u64;
    let buyback_share = claimed - to_treasury + carry;
    let buyback_costs = if volume_accumulator.lamports() == 0 {
        rent.minimum_balance(VOLUME_ACCUMULATOR_MAX_LEN)
    } else {
        0
    };
    let to_buyback = buyback_share.saturating_sub(buyback_costs);
    require_gte!(to_buyback, MIN_CLAIM_LAMPORTS, QubyteError::NothingToClaim);

    let mint_key = ctx.accounts.mint.key();
    let fee_vault_seeds: &[&[u8]] = &[FEE_VAULT_SEED, mint_key.as_ref(), &[coin.fee_vault_bump]];
    let signer = &[fee_vault_seeds];
    let pay_from_fee_vault = |to: AccountInfo<'info>, lamports: u64| {
        system_program::transfer(
            CpiContext::new_with_signer(
                system_program::ID,
                system_program::Transfer {
                    from: fee_vault.to_account_info(),
                    to,
                },
                signer,
            ),
            lamports,
        )
    };
    pay_from_fee_vault(ctx.accounts.treasury.to_account_info(), to_treasury)?;

    if venue == Venue::Pool {
        let wsol = ctx
            .accounts
            .fee_vault_wsol_account
            .as_ref()
            .ok_or(error!(QubyteError::InvalidVenue))?;
        pay_from_fee_vault(wsol.to_account_info(), to_buyback)?;
        token::sync_native(CpiContext::new(
            token::ID,
            token::SyncNative {
                account: wsol.to_account_info(),
            },
        ))?;
    }
    venue::buy(
        venue,
        venue_accounts,
        &fee_vault.key(),
        to_buyback,
        1,
        fee_vault_seeds,
    )?;

    let token_account = &mut ctx.accounts.fee_vault_token_account;
    token_account.reload()?;
    let tokens_burned = token_account.amount;
    require_gt!(tokens_burned, 0, QubyteError::NothingBought);
    token_interface::burn(
        CpiContext::new_with_signer(
            anchor_spl::token_2022::ID,
            token_interface::Burn {
                mint: ctx.accounts.mint.to_account_info(),
                from: token_account.to_account_info(),
                authority: fee_vault.to_account_info(),
            },
            signer,
        ),
        tokens_burned,
    )?;

    let balance_after = ctx.accounts.fee_vault.lamports();
    let sol_spent = balance_before
        .saturating_sub(balance_after)
        .saturating_sub(to_treasury);
    let clock = Clock::get()?;
    let coin = &mut ctx.accounts.coin_state;
    coin.buyback_carry = balance_after.saturating_sub(rent.minimum_balance(0));
    coin.total_burned = coin.total_burned.saturating_add(tokens_burned);
    coin.total_sol_spent = coin.total_sol_spent.saturating_add(sol_spent);
    coin.total_sol_to_treasury = coin.total_sol_to_treasury.saturating_add(to_treasury);
    coin.buyback_count = coin.buyback_count.saturating_add(1);
    coin.last_buyback_slot = clock.slot;
    coin.last_buyback_at = clock.unix_timestamp;
    emit!(BuybackEvent {
        mint: mint_key,
        sol_to_treasury: to_treasury,
        sol_spent,
        tokens_burned,
        total_burned: coin.total_burned,
    });
    Ok(())
}
