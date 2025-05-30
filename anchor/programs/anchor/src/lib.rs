use anchor_lang::prelude::*;

declare_id!("5BEwy1km3f87NE7tQr54os4jRbFMJHfLKMVaxm38mQ3L");

#[program]
pub mod anchor {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
