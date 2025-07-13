#![allow(clippy::result_large_err)]

use anchor_lang::prelude::*;

declare_id!("FqzkXZdwYjurnUKetJCAvaUw5WAqbwzU6gZEwydeEfqS");

#[error_code]
pub enum RafflesErrors {
    #[msg("Capacity can not be negative!")]
    NegativeCapacityNotAllowed,

    #[msg("Close date has passed!")]
    CloseDateHasPassed,
}

#[program]
pub mod raffles {
    use super::*;

    pub fn initialize_id_counter(ctx: Context<InitializeIdCounter>) -> Result<()> {
        ctx.accounts.id_counter.value = 0;
        ctx.accounts.id_counter.bump = ctx.bumps.id_counter;
        msg!("Program Initialized.");
        Ok(())
    }

    pub fn initialize_vault(ctx: Context<InitializeVault>, capacity: u32, close_at: u64) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        vault.id = ctx.accounts.current_id.value;
        vault.authority = *ctx.accounts.creator.key;
        ctx.accounts.current_id.value += 1;
        vault.participants = 0;
        vault.capacity = capacity;
        vault.bump = ctx.bumps.vault;
        vault.close_at = close_at;
        vault.created_at = ctx.accounts.clock.unix_timestamp as u64;
        require!(vault.close_at == 0 || vault.close_at > vault.created_at, RafflesErrors::CloseDateHasPassed);
        vault.pool = 0;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeIdCounter<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        init_if_needed,
        payer=user,
        seeds=[b"counter"],
        space=8 + IdCounter::INIT_SPACE,
        bump,
    )]
    pub id_counter: Account<'info, IdCounter>,

    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct InitializeVault<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        seeds=[b"id_counter",],
        bump=current_id.bump,
    )]
    pub current_id: Account<'info, IdCounter>,

    #[account(
        init,
        payer=creator,
        seeds=[b"vault", creator.key.as_ref(), current_id.value.to_le_bytes().as_ref()],
        space=8 + Vault::INIT_SPACE,
        bump
    )]
    pub vault: Account<'info, Vault>,

    pub system_program: Program<'info, System>,

    pub clock: Sysvar<'info, Clock>,
}

#[account]
#[derive(InitSpace)]
pub struct Vault {
    pub authority: Pubkey,
    pub bump: u8,
    pub pool: u64,
    pub participants: u32,
    pub capacity: u32, // zero to unlimited
    pub close_at: u64,
    pub created_at: u64,
    pub id: u64,
}

#[account]
#[derive(InitSpace)]
pub struct IdCounter {
    value: u64,
    bump: u8,
}
