pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("A4ujkLWbSZYCZMi6iwxKHrbNsyYRPvLh2vLqp58Jg8D8");

#[program]
pub mod dex {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        initialize::handler(ctx)
    }

    use super::*;

    pub fn initialize_global_escrow(context: Context<InitializeGlobalEscrow>) -> Result<()> {
        msg!("Checking if global escrow account already exists...");
        if context.accounts.global_escrow.to_account_info().lamports() > 0 {
            msg!("Global escrow account already exists, skipping initialization.");
            return Ok(());
        }

        msg!("Initializing global escrow account...");

        // Initialize the escrow account with the correct mint
        token::initialize_account(CpiContext::new(
            context.accounts.token_program.to_account_info(),
            InitializeAccount {
                account: context.accounts.global_escrow.to_account_info(),
                mint: context.accounts.token_mint.to_account_info(),
                authority: context.accounts.global_escrow_authority.to_account_info(),
                rent: context.accounts.rent.to_account_info(),
            },
        ))?;
        msg!("Global escrow account initialized.");

        Ok(())
    }

    // pub fn change_global_escrow_authority(
    //     context: Context<ChangeEscrowAuthority>,
    //     new_authority: Pubkey,
    // ) -> Result<()> {
    //     msg!("Changing global escrow authority...");

    //     token::set_authority(
    //         CpiContext::new(
    //             context.accounts.token_program.to_account_info(),
    //             SetAuthority {
    //                 account_or_mint: context.accounts.global_escrow.to_account_info(),
    //                 current_authority: context.accounts.global_escrow_authority.to_account_info(),
    //             },
    //         ),
    //         AuthorityType::AccountOwner,
    //         Some(new_authority),
    //     )?;

    //     msg!("Global escrow authority changed to: {}", new_authority);

    //     Ok(())
    // }

    pub fn deposit_seller(
        context: Context<CreateTrade>,
        trade_id: u64,
        is_native: bool,
        deposit_value: u64,
        end_time: i64,
    ) -> Result<()> {
        msg!("Greeting from deposit_seller {}", context.program_id);

        require!(deposit_value > 0, P2PError::InvalidAmount);

        let trade_account = &mut context.accounts.trade_account;

        let seller_public_key = context.accounts.seller.key();
        msg!(
            "Seller {}'s trade id is {}, is_native is {}, deposit_value is {}",
            seller_public_key,
            trade_id,
            is_native,
            deposit_value,
        );

        trade_account.trade_id = trade_id;
        trade_account.seller = context.accounts.seller.key();
        trade_account.buyer = None;
        trade_account.is_native = is_native;
        trade_account.deposit_value = deposit_value;
        trade_account.available_value = deposit_value;
        trade_account.end_time = end_time;
        trade_account.bump = context.bumps.trade_account;

        if is_native {
            // Handle native SOL transfer
            let transfer_instruction = anchor_lang::solana_program::system_instruction::transfer(
                &context.accounts.seller.key(),
                &context.accounts.global_escrow.key(),
                deposit_value,
            );
            anchor_lang::solana_program::program::invoke(
                &transfer_instruction,
                &[
                    context.accounts.seller.to_account_info(),
                    context.accounts.global_escrow.to_account_info(),
                    context.accounts.system_program.to_account_info(),
                ],
            )?;
            msg!("Native SOL transfer completed.");
        } else {
            // Handle SPL token transfer to single escrow account
            token::transfer(
                CpiContext::new(
                    context.accounts.token_program.to_account_info(),
                    token::Transfer {
                        from: context
                            .accounts
                            .seller_token_account
                            .as_ref()
                            .unwrap()
                            .to_account_info(),
                        to: context.accounts.global_escrow.to_account_info(),
                        authority: context.accounts.seller.to_account_info(),
                    },
                ),
                deposit_value,
            )?;
            msg!("SPL Token transfer to global escrow completed.");
        }

        emit!(CreateTradeEvent {
            trade_id,
            trade_account: trade_account.key(),
            seller: trade_account.seller,
            deposit_value,
            end_time
        });

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeGlobalEscrow<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        seeds = [b"global_escrow"],
        bump,
        token::mint = token_mint,
        token::authority = global_escrow_authority
    )]
    pub global_escrow: Account<'info, TokenAccount>,

    /// CHECK: This is the authority for the global escrow
    #[account(
        seeds = [b"global_escrow_authority"],
        bump
    )]
    pub global_escrow_authority: UncheckedAccount<'info>,
    #[account()]
    pub token_mint: Account<'info, Mint>,

    #[account(address = spl_token::ID)]
    pub token_program: Program<'info, Token>,

    #[account(address = anchor_lang::system_program::ID)]
    pub system_program: Program<'info, System>,

    pub rent: Sysvar<'info, Rent>,
}

// #[derive(Accounts)]
// pub struct ChangeEscrowAuthority<'info> {
//     #[account(mut)]
//     pub payer: Signer<'info>,

//     #[account(
//         mut,
//         seeds = [b"global_escrow"],
//         bump
//     )]
//     pub global_escrow: Account<'info, TokenAccount>,

//     #[account(
//         seeds = [b"global_escrow_authority"],
//         bump
//     )]
//     /// CHECK: This is the current authority PDA
//     pub global_escrow_authority: UncheckedAccount<'info>,

//     #[account(address = spl_token::ID)]
//     pub token_program: Program<'info, Token>,
// }

#[derive(Accounts)]
#[instruction(trade_id: u64)]
pub struct CreateTrade<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,

    #[account(
        init_if_needed,
        payer = seller,
        space = ANCHOR_DISCRIMINATOR_SIZE + 108 + 8, // Total: 124 bytes
        seeds = [b"trade", trade_id.to_le_bytes().as_ref()],
        bump
    )]
    pub trade_account: Account<'info, TradeAccount>,

    /// If depositing SPL tokens
    #[account(mut)]
    pub seller_token_account: Option<Account<'info, TokenAccount>>,

    /// Single escrow account for all SPL tokens
    #[account(
        mut,
        seeds = [b"global_escrow"],
        bump
    )]
    pub global_escrow: Account<'info, TokenAccount>,

    #[account()]
    pub token_mint: Option<Account<'info, Mint>>,

    #[account(address = spl_token::ID)]
    pub token_program: Program<'info, Token>,

    #[account(address = anchor_lang::system_program::ID)]
    pub system_program: Program<'info, System>,
}

#[account]
pub struct TradeAccount {
    pub trade_id: u64,
    pub seller: Pubkey,
    pub buyer: Option<Pubkey>,
    pub is_native: bool,
    pub deposit_value: u64,
    pub available_value: u64,
    pub end_time: i64,
    pub is_finalized: bool,
    pub bump: u8,
}

#[event]
pub struct CreateTradeEvent {
    #[index]
    pub trade_id: u64,
    pub trade_account: Pubkey,
    pub seller: Pubkey,
    pub deposit_value: u64,
    pub end_time: i64,
}

#[error_code]
pub enum P2PError {
    #[msg("Invalid amount or zero amount not allowed.")]
    InvalidAmount,
}
