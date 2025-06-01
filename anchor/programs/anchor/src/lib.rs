use anchor_lang::prelude::*;

declare_id!("5BEwy1km3f87NE7tQr54os4jRbFMJHfLKMVaxm38mQ3L");

#[program]
mod anchor {
    use anchor_lang::system_program;
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let stake_account = &mut ctx.accounts.stake_account;
        let clock = Clock::get()?;

        stake_account.owner = ctx.accounts.signer.key();
        stake_account.staked_amount = 0;
        stake_account.total_points = 0;
        stake_account.last_stake_time = clock.unix_timestamp;
        stake_account.bump = ctx.bumps.stake_account;

        msg!("Stake account initialized for {}", stake_account.owner);
        Ok(())
    }

    pub fn create_stake(ctx: Context<CreateStake>, amount: u64) -> Result<()> {
        require!(amount > 0, StakeError::InsufficientBalance);

        let stake_account = &mut ctx.accounts.stake_account;
        let clock = Clock::get()?;

        stake_account.staked_amount = amount;
        stake_account.owner = ctx.accounts.user.key();
        stake_account.last_stake_time = clock.unix_timestamp;

        // Update points based on the staked amount and time elapsed
        update_points(stake_account, clock.unix_timestamp)?;

        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.user.to_account_info(),
                to: stake_account.to_account_info(),
            }
        );

        system_program::transfer(cpi_context, amount)?;

        stake_account.staked_amount = stake_account.staked_amount
            .checked_add(amount)
            .ok_or(StakeError::OverFlow)?;

        msg!(
            "Staked {} lamports. Total staked: {}, Total points: {}",
            amount,
            stake_account.staked_amount,
            stake_account.total_points / 1_000_000
        );
        Ok(())
    }

    pub fn un_stake(ctx: Context<UnStake>, amount: u64) -> Result<()> {
        require!(amount > 0, StakeError::InsufficientBalance);

        let stake_account = &mut ctx.accounts.stake_account;
        let clock = Clock::get()?;

        update_points(stake_account, clock.unix_timestamp)?;

        // using seeds to sign the transaction from the pda as it doesn't has the private key to sign so we use these seeds to sign the transaction
        let binding = stake_account.owner.key();
        let seeds = &[b"user_stake", binding.as_ref(), &[stake_account.bump]];

        let signer = &[&seeds[..]];

        require!(stake_account.staked_amount > amount, StakeError::InsufficientBalance);

        let cpi_context = CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: stake_account.to_account_info(),
                to: ctx.accounts.user.to_account_info(),
            },
            signer
        );
        system_program::transfer(cpi_context, amount)?;

        stake_account.staked_amount = stake_account.staked_amount
            .checked_sub(amount)
            .ok_or(StakeError::UnderFlow)?;

        msg!(
            "Unstaked {} lamports. Remaining staked: {}, Total points: {}",
            amount,
            stake_account.staked_amount,
            stake_account.total_points / 1_000_000
        );
        Ok(())
    }

    pub fn claim_points(ctx: Context<ClaimPoints>) -> Result<()> {
        let stake_account = &mut ctx.accounts.stake_account;
        let clock = Clock::get()?;

        update_points(stake_account, clock.unix_timestamp)?;

        let points_claimable = stake_account.total_points / 1_000_000;
        msg!("user has {} claimable points", points_claimable);

        stake_account.total_points = 0;

        Ok(())
    }
}

fn update_points(account: &mut StakeAccount, current_time: i64) -> Result<()> {
    let staked_amount = account.staked_amount;
    let time_elapsed = current_time
        .checked_sub(account.last_stake_time)
        .ok_or(StakeError::InvalidTimeStamp)? as u64;

    if time_elapsed > 0 && staked_amount > 0 {
        let new_points = calculate_points(staked_amount, time_elapsed)?;
        account.total_points = account.total_points
            .checked_add(new_points)
            .ok_or(StakeError::OverFlow)?;
    }
    account.last_stake_time = current_time;
    Ok(())
}

fn calculate_points(staked_amount: u64, time_elapsed: u64) -> Result<u64> {
    let points = (staked_amount as u128)
        .checked_mul(time_elapsed as u128)
        .ok_or(StakeError::InvalidTimeStamp)?
        .checked_mul(1_000_000)
        .ok_or(StakeError::OverFlow)?
        .checked_div(1_000_000)
        .ok_or(StakeError::OverFlow)?
        .checked_div(86400)
        .ok_or(StakeError::OverFlow)?;

    Ok(points as u64)
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(
        init,
        payer = signer,
        space = 8 + 32 + 8 + 8 + 8 + 1,
        seeds = [b"stake_account", signer.key().as_ref()],
        bump
    )]
    pub stake_account: Account<'info, StakeAccount>,

    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
pub struct CreateStake<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"user_stake",user.key().as_ref()],
        bump,
        constraint = stake_account.owner == user.key() @ StakeError::UnAuthorised
    )]
    pub stake_account: Account<'info, StakeAccount>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UnStake<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"user_stake",user.key().as_ref()],
        bump,
        constraint = stake_account.owner == user.key() @ StakeError::UnAuthorised
    )]
    pub stake_account: Account<'info, StakeAccount>,

    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
pub struct ClaimPoints<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"user_stake",user.key().as_ref()],
        bump,
        constraint = stake_account.owner == user.key() @ StakeError::UnAuthorised
    )]
    pub stake_account: Account<'info, StakeAccount>,
}

#[account]
pub struct StakeAccount {
    pub owner: Pubkey,
    pub staked_amount: u64,
    pub total_points: u64,
    pub last_stake_time: i64,
    pub bump: u8,
}

#[error_code]
pub enum StakeError {
    #[msg("Amount must be greater than zero")]
    InsufficientBalance,
    #[msg("Invalid Timestamp")]
    InvalidTimeStamp,
    #[msg("Arithmatic overflow")]
    OverFlow,
    #[msg("Arithmatic underflow")]
    UnderFlow,
    #[msg("You are not allowed")]
    UnAuthorised,
}
