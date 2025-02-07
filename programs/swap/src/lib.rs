use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, Token, TokenAccount, Transfer},
};

//program_id
declare_id!("B7TKWwRjzkTKb9kc5VSA9gawufx6SmbkViyx2GmMFtWL");

#[program]
pub mod swap {
    use super::*;

    /// For depositing **raw SOL** into a System-owned vault.
    pub fn deposit_seller_native(
        ctx: Context<MakeOfferNative>,
        id: u64,
        token_b_wanted_amount: u64,
        sol_offered_amount: u64,
        is_taker_native: bool,
    ) -> Result<()> {
        require!(sol_offered_amount > 0, P2PError::InvalidAmount);

        // Transfer SOL from maker to the system-owned vault.
        let transfer_ix = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.maker.key(),
            &ctx.accounts.vault.key(),
            sol_offered_amount,
        );

        anchor_lang::solana_program::program::invoke_signed(
            &transfer_ix,
            &[
                ctx.accounts.maker.to_account_info(),
                ctx.accounts.vault.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            // &[&[b"vault-native", &[ctx.bumps.vault]]],
            &[], // No signer seeds needed for maker’s signature
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
            token_a_offered_amount: sol_offered_amount,
            token_b_wanted_amount,
            is_native: true,
            is_taker_native,
            is_swap_completed: false,
            bump: ctx.bumps.offer,
        });

        emit!(CreateTradeEvent {
            id,
            maker: ctx.accounts.maker.key(),
            token_a_offered_amount: sol_offered_amount,
            token_b_wanted_amount,
            is_taker_native,
            is_swap_completed: false,
        });

        Ok(())
    }

    /// For depositing **SPL tokens** into an ATA vault.
    pub fn deposit_seller_spl(
        ctx: Context<MakeOfferSpl>,
        id: u64,
        token_b_wanted_amount: u64,
        token_a_offered_amount: u64,
        is_taker_native: bool,
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
            token_a_offered_amount,
            token_b_wanted_amount,
            is_native: false,
            is_taker_native,
            is_swap_completed: false,
            bump: ctx.bumps.offer,
        });

        emit!(CreateTradeEvent {
            id,
            maker: ctx.accounts.maker.key(),
            token_a_offered_amount,
            token_b_wanted_amount,
            is_taker_native,
            is_swap_completed: false,
        });

        Ok(())
    }

    pub fn take_offer(ctx: Context<TakeOffer>, id: u64) -> Result<()> {
        let offer = &mut ctx.accounts.offer;

        // Ensure the offer has not already been filled.
        require!(!offer.is_swap_completed, P2PError::SwapAlreadyCompleted);
        // Prevent a maker from filling their own offer.
        require!(
            offer.maker != ctx.accounts.taker.key(),
            P2PError::MakerAndTakerCannotBeSame
        );

        if offer.is_native {
            let vault_bump = ctx.bumps.vault_native;
            let seeds: &[&[u8]] = &[b"vault-native", &[vault_bump]];
            let signer_seeds = &[&seeds[..]];

            // -------------------------------
            let transfer_ix = anchor_lang::solana_program::system_instruction::transfer(
                &ctx.accounts.vault_native.key(),
                &ctx.accounts.taker.key(),
                offer.token_a_offered_amount,
            );

            anchor_lang::solana_program::program::invoke_signed(
                &transfer_ix,
                &[
                    ctx.accounts.vault_native.to_account_info(),
                    ctx.accounts.taker.to_account_info(),
                    ctx.accounts.system_program.to_account_info(),
                ],
                signer_seeds,
            )?;

            msg!(
                "Native SOL transferred {} lamports from native vault to taker.",
                offer.token_a_offered_amount
            );
        } else {
            // Use the global authority PDA to sign for the vault.
            let global_authority_seeds =
                &[b"global-authority".as_ref(), &[ctx.bumps.global_authority]];

            let signer_seeds = [&global_authority_seeds[..]];

            token::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.vault_spl.to_account_info(),
                        to: ctx.accounts.taker_token_account_a.to_account_info(),
                        authority: ctx.accounts.global_authority.to_account_info(),
                    },
                    &signer_seeds,
                ),
                offer.token_a_offered_amount,
            )?;

            msg!(
                "SPL tokens transferred from vault to taker: {} tokens",
                offer.token_a_offered_amount
            );
        }

        // -------------------------------
        // Step 2: Transfer Taker's asset to Maker.

        if offer.is_taker_native {
            // Transfer SOL from taker to maker.
            // The taker is the signer for the transfer.

            let transfer_ix = anchor_lang::solana_program::system_instruction::transfer(
                &ctx.accounts.taker.key(),
                &ctx.accounts.maker.key(),
                offer.token_b_wanted_amount,
            );

            anchor_lang::solana_program::program::invoke_signed(
                &transfer_ix,
                &[
                    ctx.accounts.taker.to_account_info(),
                    ctx.accounts.maker.to_account_info(),
                    ctx.accounts.system_program.to_account_info(),
                ],
                &[],
            )?;

            msg!(
                "Native SOL transferred {} lamports from taker to maker.",
                offer.token_b_wanted_amount
            );
        } else {
            token::transfer(
                CpiContext::new(
                    ctx.accounts.token_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.taker_token_account_b.to_account_info(),
                        to: ctx.accounts.maker_token_account_b.to_account_info(),
                        authority: ctx.accounts.taker.to_account_info(),
                    },
                ),
                offer.token_b_wanted_amount,
            )?;

            msg!(
                "SPL tokens transferred from taker to maker: {} tokens",
                offer.token_b_wanted_amount
            );
        }

        // if you don't want on-chain state tracking, you can remove this line and just emit the event and close the offer account.
        offer.is_swap_completed = true;

        emit!(SwapCompletedEvent {
            id: offer.id,
            maker: offer.maker,
            taker: ctx.accounts.taker.key(),
            token_a_transferred: offer.token_a_offered_amount,
            token_b_transferred: offer.token_b_wanted_amount,
            is_swap_completed: true,
        });

    
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(id: u64)]
pub struct TakeOffer<'info> {
    #[account(mut)]
    pub taker: Signer<'info>,

    #[account(mut)]
    pub maker: SystemAccount<'info>,

    #[account()]
    pub token_mint_a: Account<'info, Mint>,

    #[account()]
    pub token_mint_b: Account<'info, Mint>,

    #[account(
        mut,
        close = maker,  // This tells Anchor to close the offer account and send its lamports to the maker.
        seeds = [b"offer", maker.key().as_ref(), id.to_le_bytes().as_ref()],
        bump
    )]
    pub offer: Account<'info, Offer>,
    /// CHECK: This is a PDA used as the authority for the global vault.
    #[account(
        mut,
        seeds = [b"vault-native"],
        bump
    )]
    pub vault_native: AccountInfo<'info>,

    #[account(
        mut,
        associated_token::mint = token_mint_a,
        associated_token::authority = global_authority,
    )]
    pub vault_spl: Account<'info, TokenAccount>,

    /// CHECK: This is a PDA used as the authority for the global vault.
    /// It does not need additional validation because it's derived using `seeds = [b"global-authority"]`.
    #[account(
        mut,
        seeds = [b"global-authority"],
        bump
    )]
    pub global_authority: AccountInfo<'info>,

    /// Taker's associated token account for receiving maker's SPL deposit.
    #[account(
        mut,
        associated_token::mint = token_mint_a,
        associated_token::authority = taker,
    )]
    pub taker_token_account_a: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = token_mint_b,
        associated_token::authority = taker,
    )]
    pub taker_token_account_b: Account<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = token_mint_b,
        token::authority = maker
    )]
    pub maker_token_account_b: Account<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
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
        seeds = [b"offer", maker.key().as_ref(), id.to_le_bytes().as_ref()],
        bump
    )]
    pub offer: Account<'info, Offer>,

    /// A simple system-owned vault account to store raw SOL
    // #[account(
    //     init_if_needed,
    //     payer = maker,
    //     space = 16, // if you only store the VaultAccount's bump or minimal data
    //     seeds = [b"vault-native"],
    //     bump
    // )]
    // pub vault: Account<'info, GlobalSolVault>,

    /// CHECK: This is a PDA used as the authority for the global vault.
    /// The global native vault.
    /// **Important:** This vault is a system account with ZERO data (space = 0)
    /// so that the system transfer instruction does not error.
    #[account(
        init_if_needed,
        payer = maker,
        space = 0,
        seeds = [b"vault-native"],
        bump,
        owner = system_program::ID
    )]
    pub vault: AccountInfo<'info>, // Program-owned account

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
        seeds = [b"offer", maker.key().as_ref(), id.to_le_bytes().as_ref()],
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

// /// Single global SOL vault data.
// #[account]
// pub struct GlobalSolVault {
//     pub bump: u8, // Could store more fields if you want
// }

/// Offer data.
#[account]
pub struct Offer {
    pub id: u64,
    pub maker: Pubkey,
    pub token_mint_a: Pubkey,
    pub token_mint_b: Pubkey,
    pub token_a_offered_amount: u64,
    pub token_b_wanted_amount: u64,
    pub is_native: bool,
    pub is_taker_native: bool,
    pub is_swap_completed: bool,
    pub bump: u8,
}

impl Offer {
    /// Helper for sizing. Adjust if you add more fields.
    pub const SIZE: usize = 8   // id
        + 32                    // maker
        + 32                    // token_mint_a
        + 32                    // token_mint_b
        + 8                     // token_a_offered_amount
        + 8                     // token_b_wanted_amount
        + 1                     // is_native
        + 1
        + 1                     // is_swap_completed
        + 1; // is_taker_nativ
}

/// Event emitted when a trade is created.
#[event]
pub struct CreateTradeEvent {
    #[index]
    pub id: u64,
    pub maker: Pubkey,
    pub token_a_offered_amount: u64,
    pub token_b_wanted_amount: u64,
    pub is_taker_native: bool,
    is_swap_completed: bool,
}

/// Event emitted when a trade is completed.
#[event]
pub struct SwapCompletedEvent {
    #[index]
    pub id: u64,
    pub maker: Pubkey,
    pub taker: Pubkey,
    pub token_a_transferred: u64,
    pub token_b_transferred: u64,
    pub is_swap_completed: bool,
}

#[error_code]
pub enum P2PError {
    #[msg("Invalid amount or zero amount not allowed.")]
    InvalidAmount,
    #[msg("Swap is already completed.")]
    SwapAlreadyCompleted,
    #[msg("Maker and taker cannot be the same.")]
    MakerAndTakerCannotBeSame,
}
