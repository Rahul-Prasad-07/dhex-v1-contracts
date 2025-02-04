use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, Token, TokenAccount, Transfer},
};

declare_id!("8Mi1JhPQHTHHaLFhLUMLZGRbVMc9D4sZtUubSRpxhpNP");

/// Program entrypoint
#[program]
pub mod swap {
    use super::*;

    /// For depositing **raw SOL** into a System-owned vault.
    pub fn deposit_seller_native(
        ctx: Context<MakeOfferNative>,
        id: u64,
        token_b_wanted_amount: u64,
        sol_offered_amount: u64,
    ) -> Result<()> {
        require!(sol_offered_amount > 0, P2PError::InvalidAmount);

        // Transfer SOL from maker to the system-owned vault.
        let transfer_ix = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.maker.key(),
            &ctx.accounts.vault.key(),
            sol_offered_amount,
        );
        anchor_lang::solana_program::program::invoke(
            &transfer_ix,
            &[
                ctx.accounts.maker.to_account_info(),
                ctx.accounts.vault.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;
        msg!(
            "Native SOL transfer completed. Moved {} lamports into vault.",
            sol_offered_amount
        );

        // Populate Offer data
        ctx.accounts.offer.set_inner(Offer {
            id,
            maker: ctx.accounts.maker.key(),
            token_mint_a: ctx.accounts.token_mint_a.key(),
            token_mint_b: ctx.accounts.token_mint_b.key(),
            token_b_wanted_amount,
            // Mark it as native just for clarity if you want
            is_native: true,
            bump: ctx.bumps.offer,
        });

        emit!(CreateTradeEvent {
            id,
            maker: ctx.accounts.maker.key(),
            token_a_offered_amount: sol_offered_amount,
            token_b_wanted_amount
        });

        Ok(())
    }

    /// For depositing **SPL tokens** into an ATA vault.
    pub fn deposit_seller_spl(
        ctx: Context<MakeOfferSpl>,
        id: u64,
        token_b_wanted_amount: u64,
        token_a_offered_amount: u64,
    ) -> Result<()> {
        require!(token_a_offered_amount > 0, P2PError::InvalidAmount);

        // Transfer SPL tokens from the maker's token account to the vault (ATA owned by Offer).
        // We can use the anchor_spl::token::transfer CPI:
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.maker_token_account_a.to_account_info(),
                    to: ctx.accounts.vault_spl.to_account_info(),
                    authority: ctx.accounts.maker.to_account_info(),
                },
            ),
            token_a_offered_amount,
        )?;
        msg!(
            "SPL token transfer completed: {} tokens moved into vault.",
            token_a_offered_amount
        );

        // Populate Offer data
        ctx.accounts.offer.set_inner(Offer {
            id,
            maker: ctx.accounts.maker.key(),
            token_mint_a: ctx.accounts.token_mint_a.key(),
            token_mint_b: ctx.accounts.token_mint_b.key(),
            token_b_wanted_amount,
            is_native: false,
            bump: ctx.bumps.offer,
        });

        emit!(CreateTradeEvent {
            id,
            maker: ctx.accounts.maker.key(),
            token_a_offered_amount,
            token_b_wanted_amount
        });

        Ok(())
    }
}

/// Context for depositing raw SOL
#[derive(Accounts)]
#[instruction(id: u64)]
pub struct MakeOfferNative<'info> {
    /// Person who deposits the SOL
    #[account(mut)]
    pub maker: Signer<'info>,

    /// Just pass in any valid mint for 'A' and 'B' if you like.  
    /// Even if `is_native`, we’re not transferring SPL here.  
    #[account()]
    pub token_mint_a: Account<'info, Mint>,

    #[account()]
    pub token_mint_b: Account<'info, Mint>,

    /// The Offer PDA storing trade details
    #[account(
        init,
        payer = maker,
        // space: big enough for all fields
        space = 8 + Offer::SIZE,
        seeds = [b"offer-native", maker.key().as_ref(), id.to_le_bytes().as_ref()],
        bump
    )]
    pub offer: Account<'info, Offer>,

    /// A simple system-owned vault account to store raw SOL
    #[account(
        init_if_needed,
        payer = maker,
        space = 16, // if you only store the VaultAccount's bump or minimal data
        seeds = [b"vault-native"],
        bump
    )]
    pub vault: Account<'info, GlobalSolVault>,

    pub system_program: Program<'info, System>,
}

/// Context for depositing SPL tokens
#[derive(Accounts)]
#[instruction(id: u64)]
pub struct MakeOfferSpl<'info> {
    /// Person who deposits the SPL tokens
    #[account(mut)]
    pub maker: Signer<'info>,

    /// The SPL mint for the tokens being offered
    #[account()]
    pub token_mint_a: Account<'info, Mint>,

    /// The other token (B) wanted
    #[account()]
    pub token_mint_b: Account<'info, Mint>,

    /// The maker's ATA holding their tokens to be offered
    #[account(
        mut,
        token::mint = token_mint_a,
        token::authority = maker
    )]
    pub maker_token_account_a: Account<'info, TokenAccount>,

    /// The Offer PDA storing trade details
    #[account(
        init,
        payer = maker,
        // space: big enough for all fields
        space = 8 + Offer::SIZE,
        seeds = [b"offer-spl", maker.key().as_ref(), id.to_le_bytes().as_ref()],
        bump
    )]
    pub offer: Account<'info, Offer>,

    #[account(
        init_if_needed,
        payer = maker,
        associated_token::mint = token_mint_a,
        associated_token::authority = global_authority,
        //associated_token::allow_owner_off_curve = true  // <--- allow PDA as authority
    )]
    pub vault_spl: Account<'info, TokenAccount>,

    /// CHECK: This is a PDA used as the authority for the global vault.
    /// It does not need additional validation because it's derived using `seeds = [b"global-authority"]`.
    #[account(
        seeds = [b"global-authority"], // ✅ Fixed PDA seed
        bump
    )]
    pub global_authority: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

/// Single global SOL vault data.
#[account]
pub struct GlobalSolVault {
    pub bump: u8, // Could store more fields if you want
}

/// Primary Offer account storing all trade data
#[account]
pub struct Offer {
    pub id: u64,
    pub maker: Pubkey,
    pub token_mint_a: Pubkey,
    pub token_mint_b: Pubkey,
    pub token_b_wanted_amount: u64,
    pub is_native: bool,
    pub bump: u8,
}

impl Offer {
    /// Helper for sizing. Adjust if you add more fields.
    pub const SIZE: usize = 8   // id
        + 32                    // maker
        + 32                    // token_mint_a
        + 32                    // token_mint_b
        + 8                     // token_b_wanted_amount
        + 1                     // is_native
        + 1
        +8; // bump
}

#[event]
pub struct CreateTradeEvent {
    #[index]
    pub id: u64,
    pub maker: Pubkey,
    pub token_a_offered_amount: u64,
    pub token_b_wanted_amount: u64,
}

#[error_code]
pub enum P2PError {
    #[msg("Invalid amount or zero amount not allowed.")]
    InvalidAmount,
}
