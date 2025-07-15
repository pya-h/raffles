#![allow(clippy::result_large_err)]

use anchor_lang::prelude::*;

declare_id!("FqzkXZdwYjurnUKetJCAvaUw5WAqbwzU6gZEwydeEfqS");

#[error_code]
pub enum RafflesErrors {
    #[msg("Capacity can not be negative!")]
    NegativeCapacityNotAllowed,

    #[msg("Close date has passed!")]
    CloseDateHasPassed,

    #[msg("Insufficient funds!")]
    InsufficientFunds,

    #[msg("This vault does not allow any more participants!")]
    CapacityExceeded,
}

#[program]
pub mod raffles {
    use anchor_lang::solana_program::example_mocks::solana_sdk::system_program;

    use super::*;

    pub fn initialize_id_counter(ctx: Context<InitializeIdCounter>) -> Result<()> {
        ctx.accounts.id_counter.value = 0;
        ctx.accounts.id_counter.bump = ctx.bumps.id_counter;
        msg!("Program Initialized.");
        Ok(())
    }

    pub fn initialize_vault(
        ctx: Context<InitializeVault>,
        capacity: u32,
        close_at: u64,
    ) -> Result<()> {
        let vault: &mut Account<'_, Vault> = &mut ctx.accounts.vault_info;
        vault.id = ctx.accounts.current_id.value;
        vault.authority = *ctx.accounts.creator.key;
        ctx.accounts.current_id.value += 1;
        vault.participants = 0;
        vault.capacity = capacity;
        vault.bump = ctx.bumps.vault_info;
        vault.close_at = close_at;
        vault.created_at = ctx.accounts.clock.unix_timestamp as u64;
        require!(
            vault.close_at == 0 || vault.close_at > vault.created_at,
            RafflesErrors::CloseDateHasPassed
        );
        vault.pool = 0;

        let (vault_inventory_pda, inventory_bump) = Pubkey::find_program_address(
            &[
                b"vault",
                ctx.accounts.creator.key.as_ref(),
                ctx.accounts.current_id.value.to_le_bytes().as_ref(),
            ],
            &ctx.program_id,
        );

        let min_lamports = Rent::get()?.minimum_balance(0);
        let cpi_accounts = anchor_lang::solana_program::system_instruction::create_account(
            ctx.accounts.creator.key,
            &vault_inventory_pda,
            min_lamports,
            0,
            &system_program::ID,
        );

        anchor_lang::solana_program::program::invoke_signed(
            &cpi_accounts,
            &[
                ctx.accounts.creator.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            &[&[
                b"vault",
                ctx.accounts.creator.key.as_ref(),
                ctx.accounts.current_id.value.to_le_bytes().as_ref(),
                &[inventory_bump],
            ]],
        )?;
        vault.inventory = vault_inventory_pda;
        Ok(())
    }

    pub fn particpate_inf_raffle(
        ctx: Context<ParticipateInRaffle>,
        _vault_id: u64,
        amount: u64,
    ) -> Result<()> {
        let vault_info = &mut ctx.accounts.vault_info;
        require!(
            vault_info.close_at == 0
                || vault_info.close_at > ctx.accounts.clock.unix_timestamp as u64,
            RafflesErrors::CloseDateHasPassed
        );

        require!(
            ctx.accounts.user.lamports() >= amount,
            RafflesErrors::InsufficientFunds
        );

        let user_share = &mut ctx.accounts.user_share;

        if user_share.amount == 0 {
            require!(
                vault_info.capacity == 0 || vault_info.participants < vault_info.capacity,
                RafflesErrors::CapacityExceeded
            );
            user_share.owner = *ctx.accounts.user.key;
            user_share.vault = vault_info.key();
            user_share.bump = ctx.bumps.user_share;
            vault_info.participants += 1;
        }

        let tx = anchor_lang::solana_program::system_instruction::transfer(&ctx.accounts.user.key(), &ctx.accounts.vault_inventory.key(), amount);

        anchor_lang::solana_program::program::invoke(&tx, &[
            ctx.accounts.user.to_account_info(),
            ctx.accounts.vault_inventory.to_account_info(),
            vault_info.to_account_info(),
        ])?; // check last 2 lines

        vault_info.pool += amount;
        user_share.amount += amount;
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

    pub system_program: Program<'info, System>,
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
        seeds=[b"vault-info", current_id.value.to_le_bytes().as_ref()],
        space=8 + Vault::INIT_SPACE,
        bump
    )]
    pub vault_info: Account<'info, Vault>,

    // #[account(
    //     init,
    //     payer = creator,
    //     space = 0,
    //     seeds = [b"vault", current_id.value.to_le_bytes().as_ref()],
    //     bump
    // )]
    // pub vault: Account<'info, Vault>,
    pub system_program: Program<'info, System>,

    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
#[instruction(vault_id: u64)]
pub struct ParticipateInRaffle<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds=[b"vault-info", vault_id.to_le_bytes().as_ref()],
        bump=vault_info.bump,
    )]
    pub vault_info: Account<'info, Vault>,

    #[account(
        init_if_needed,
        payer=user,
        space=8 + VaultShare::INIT_SPACE,
        seeds=[b"shares", vault_info.key().as_ref(), user.key.as_ref()],
        bump,
    )]
    pub user_share: Account<'info, VaultShare>,

    #[account(
        mut, 
        seeds=[b"vault", vault_info.authority.as_ref(), vault_info.id.to_le_bytes().as_ref()], 
        bump
    )]
    pub vault_inventory: AccountInfo<'info>,

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
    pub inventory: Pubkey,
}

#[account]
#[derive(InitSpace)]
pub struct VaultShare {
    pub owner: Pubkey,
    pub vault: Pubkey,
    pub amount: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct IdCounter {
    value: u64,
    bump: u8,
}
