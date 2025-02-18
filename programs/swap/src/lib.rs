use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, Token, TokenAccount, Transfer},
};

//program_id
declare_id!("2UQCGKeeDf5YpLHCz6oLMP2j4cN2P2snRoZosGe28Aot");

#[program]
pub mod swap {
    use super::*;
    /// Intra-Chain
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

    /// Interchain => Origin is SOL chain
    /// For depositing **raw SOL** into a System-owned vault.
    /// ID: trade id, offered_amount, wanted_amount, is_taker_native,
    pub fn interchain_origin_sol_deposit_seller_native(
        ctx: Context<InterchainOriginSolMakeOfferNative>,
        id: u64,
        // external_seller_sol: Pubkey,
        seller_evm: [u8; 20], // EVM address
        token_b_wanted_amount: u64,
        sol_offered_amount: u64,
        is_taker_native: bool,
    ) -> Result<()> {
        require!(sol_offered_amount > 0, P2PError::InvalidAmount);

        // Transfer SOL from maker to the system-owned vault.
        let transfer_ix = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.seller_sol.key(),
            &ctx.accounts.vault.key(),
            sol_offered_amount,
        );

        anchor_lang::solana_program::program::invoke_signed(
            &transfer_ix,
            &[
                ctx.accounts.seller_sol.to_account_info(),
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

        // Populate Offer data individually so that other fns can set the data
        // i just wants update two field from existing offer account and all other fields are same
        // update buyer_sol and buyer_evm field
        let offer = &mut ctx.accounts.interchain_origin_sol_offer;
        offer.trade_id = id;
        offer.seller_sol = ctx.accounts.seller_sol.key();
        offer.seller_evm = seller_evm;
        offer.token_a_offered_amount = sol_offered_amount;
        offer.token_b_wanted_amount = token_b_wanted_amount;
        offer.is_seller_origin_sol = true;
        offer.is_taker_native = is_taker_native;
        offer.is_swap_completed = false;
        offer.is_native = true;
        offer.bump = ctx.bumps.interchain_origin_sol_offer;

        emit!(InterchainOriginSolCreateTradeEvent {
            id,
            seller_sol: ctx.accounts.seller_sol.key(),
            seller_evm,
            token_a_offered_amount: sol_offered_amount,
            token_b_wanted_amount,
            is_taker_native,
            is_native: true,
            is_swap_completed: false,
        });

        Ok(())
    }

    /// Interchain => Origin is EVM chain
    pub fn interchain_deposit_seller_native(
        ctx: Context<InterchainMakeOfferNative>,
        id: u64,
        external_seller_sol: Pubkey,
        buyer_evm: [u8; 20], // EVM address
        token_b_wanted_amount: u64,
        sol_offered_amount: u64,
        is_taker_native: bool,
    ) -> Result<()> {
        require!(sol_offered_amount > 0, P2PError::InvalidAmount);

        // Transfer SOL from maker to the system-owned vault.
        let transfer_ix = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.buyer_sol.key(),
            &ctx.accounts.vault.key(),
            sol_offered_amount,
        );

        anchor_lang::solana_program::program::invoke_signed(
            &transfer_ix,
            &[
                ctx.accounts.buyer_sol.to_account_info(),
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

        // Populate Offer data individually so that other fns can set the data
        // i just wants update two field from existing offer account and all other fields are same
        // update buyer_sol and buyer_evm field
        let offer = &mut ctx.accounts.offer;
        offer.buyer_sol = ctx.accounts.buyer_sol.key();
        offer.buyer_evm = buyer_evm;

        emit!(InterchainCreateTradeEvent {
            id,
            buyer: ctx.accounts.buyer_sol.key(),
            token_a_offered_amount: sol_offered_amount,
            token_b_wanted_amount,
            is_taker_native,
            is_swap_completed: false,
        });

        Ok(())
    }

    /// Intra-Chain
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

    /// Interchain => Origin is SOl chain
    pub fn interchain_origin_sol_deposit_seller_spl(
        ctx: Context<InterchainOriginSolMakeOfferSpl>,
        id: u64,
        seller_evm: [u8; 20], // EVM address
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
                    from: ctx.accounts.seller_sol_token_account_a.to_account_info(),
                    to: ctx.accounts.vault_spl.to_account_info(),
                    authority: ctx.accounts.seller_sol.to_account_info(),
                },
            ),
            token_a_offered_amount,
        )?;
        msg!(
            "SPL token transfer completed: {} tokens moved into vault.",
            token_a_offered_amount
        );

        // Populate Offer data individually so that other fns can set the data
        // i just wants update two field from existing offer account and all other fields are same
        // update buyer_sol and buyer_evm field
        let offer = &mut ctx.accounts.interchain_origin_sol_offer;
        offer.trade_id = id;
        offer.seller_sol = ctx.accounts.seller_sol.key();
        offer.seller_evm = seller_evm;
        offer.token_a_offered_amount = token_a_offered_amount;
        offer.token_b_wanted_amount = token_b_wanted_amount;
        offer.is_seller_origin_sol = true;
        offer.is_taker_native = is_taker_native;
        offer.is_swap_completed = false;
        offer.is_native = false;
        offer.bump = ctx.bumps.interchain_origin_sol_offer;

        emit!(InterchainOriginSolCreateTradeEvent {
            id,
            seller_sol: ctx.accounts.seller_sol.key(),
            seller_evm,
            token_a_offered_amount: token_a_offered_amount,
            token_b_wanted_amount,
            is_taker_native,
            is_native: true,
            is_swap_completed: false,
        });
        Ok(())
    }

    /// Interchain => Origin is EVM chain
    pub fn interchain_deposit_seller_spl(
        ctx: Context<InterchainMakeOfferSpl>,
        id: u64,
        external_seller_sol: Pubkey,
        buyer_evm: [u8; 20], // EVM address
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
                    from: ctx.accounts.buyer_sol_token_account_a.to_account_info(),
                    to: ctx.accounts.vault_spl.to_account_info(),
                    authority: ctx.accounts.buyer_sol.to_account_info(),
                },
            ),
            token_a_offered_amount,
        )?;
        msg!(
            "SPL token transfer completed: {} tokens moved into vault.",
            token_a_offered_amount
        );

        // Populate Offer data individually so that other fns can set the data
        // i just wants update two field from existing offer account and all other fields are same
        // update buyer_sol and buyer_evm field
        let offer = &mut ctx.accounts.offer;
        offer.buyer_sol = ctx.accounts.buyer_sol.key();
        offer.buyer_evm = buyer_evm;

        emit!(InterchainCreateTradeEvent {
            id,
            buyer: ctx.accounts.buyer_sol.key(),
            token_a_offered_amount: token_a_offered_amount,
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

    // if the **origin** is EVM chains,
    // this is a clone of the relay_offer function
    // this fn is called maker deposit assets on EVM chain and we relay the offer to the Solana chain
    // this fn is executed by listening a event "tradeCreated" on EVM chain
    pub fn relay_offer_clone(
        ctx: Context<RelayOfferClone>,
        id: u64,
        external_seller_evm: [u8; 20], // EVM address
        external_seller_sol: Pubkey,   // Solana address
        token_a_offered_amount: u64,
        token_b_wanted_amount: u64,
        is_taker_native: bool,
        chain_id: u64,
    ) -> Result<()> {
        // Populate InterchainOffer data using set_inner
        // ctx.accounts.interchain_offer.set_inner(InterchainOffer {
        //     trade_id: id,
        //     external_seller_sol,
        //     external_seller_evm,
        //     is_seller_origin_sol: false,
        //     is_taker_native,
        //     is_swap_completed: false,
        //     is_native: false,
        //     chain_id,
        //     token_a_offered_amount,
        //     token_b_wanted_amount,
        //     token_mint_a: ctx.accounts.token_mint_a.key(),
        //     fee_collected: 0,
        //     bump: ctx.bumps.interchain_offer,
        // });

        // Populate InterchainOffer data individually so that other fns can set the data
        let interchain_offer = &mut ctx.accounts.interchain_offer;
        interchain_offer.trade_id = id;
        interchain_offer.external_seller_sol = external_seller_sol;
        interchain_offer.external_seller_evm = external_seller_evm;
        interchain_offer.is_seller_origin_sol = false;
        interchain_offer.is_taker_native = is_taker_native;
        interchain_offer.is_swap_completed = false;
        interchain_offer.is_native = false;
        interchain_offer.chain_id = chain_id;
        interchain_offer.token_a_offered_amount = token_a_offered_amount;
        interchain_offer.token_b_wanted_amount = token_b_wanted_amount;
        interchain_offer.token_mint_a = ctx.accounts.token_mint_a.key();
        interchain_offer.fee_collected = 0;
        interchain_offer.bump = ctx.bumps.interchain_offer;

        // emit an event
        // this event will be listened by the Solana chain
        // and the Solana chain will create a new offer account
        // and the taker will take the offer
        emit!(RelayEvmTradeEvent {
            trade_id: id,
            external_seller_sol,
            external_seller_evm,
            is_seller_origin_sol: false,
            is_taker_native,
            is_swap_completed: false,
            is_native: false,
            chain_id,
            token_a_offered_amount,
            token_b_wanted_amount,
            token_mint_a: ctx.accounts.token_mint_a.key(),
            fee_collected: 0,
        });

        msg!(
            "Relay offer completed for trade id: {}, external_seller_sol: {}, 
        external_seller_evm : {:?}, is_swap_completed: {}, token_a_offered_amount: {},token_b_wanted_amount: {}, is_taker_native: {}",
            id,
            external_seller_sol,
            external_seller_evm,
            false,
            token_a_offered_amount,
            token_b_wanted_amount,
            is_taker_native
        );

        Ok(())
    }

    pub fn finalize_interchain_offer(ctx: Context<TakeInterchainOffer>, id: u64) -> Result<()> {
        let offer = &mut ctx.accounts.offer;

        // Ensure the offer has not already been filled.
        require!(!offer.is_swap_completed, P2PError::SwapAlreadyCompleted);
        // Prevent a maker from filling their own offer.
        require!(
            offer.buyer_sol != ctx.accounts.external_seller_sol.key(),
            P2PError::MakerAndTakerCannotBeSame
        );

        msg!(
            "Finalizing cross-chain swap: {} USDT from vault to seller {}",
            offer.token_b_wanted_amount,
            ctx.accounts.external_seller_sol.key()
        );

        // -------------------------------
        // Step 1: Transfer Buyer's asset from vault to seller. for origin is EVM chain
        // step 1: transfer seller's asset from vault to buyer for origin is SOL chain

        if offer.is_native {
            let vault_bump = ctx.bumps.vault_native;
            let seeds: &[&[u8]] = &[b"vault-native", &[vault_bump]];
            let signer_seeds = &[&seeds[..]];

            // -------------------------------
            let transfer_ix = anchor_lang::solana_program::system_instruction::transfer(
                &ctx.accounts.vault_native.key(),
                &ctx.accounts.external_seller_sol.key(),
                offer.token_b_wanted_amount,
            );

            anchor_lang::solana_program::program::invoke_signed(
                &transfer_ix,
                &[
                    ctx.accounts.vault_native.to_account_info(),
                    ctx.accounts.external_seller_sol.to_account_info(),
                    ctx.accounts.system_program.to_account_info(),
                ],
                signer_seeds,
            )?;

            msg!(
                "Native SOL transferred {} lamports from native vault to seller.",
                offer.token_b_wanted_amount
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
                        to: ctx
                            .accounts
                            .external_seller_sol_token_account_a
                            .to_account_info(),
                        authority: ctx.accounts.global_authority.to_account_info(),
                    },
                    &signer_seeds,
                ),
                offer.token_b_wanted_amount,
            )?;

            msg!(
                "SPL tokens transferred from vault to taker: {} tokens",
                offer.token_a_offered_amount
            );
        }

        // if you don't want on-chain state tracking, you can remove this line and just emit the event and close the offer account.
        offer.is_swap_completed = true;

        // emit logs
        msg!(
            "Interchain offer completed for trade id: {},buyer : {} , external_seller_sol: {},
        external_seller_evm : {:?}, is_swap_completed: {}, token_a_offered_amount: {},token_b_wanted_amount: {}, is_taker_native: {}",
            offer.trade_id,
            offer.buyer_sol,
            offer.external_seller_sol,
            offer.external_seller_evm,
            offer.is_swap_completed,
            offer.token_a_offered_amount,
            offer.token_b_wanted_amount,
            offer.is_taker_native
        );

        emit!(InterchainSwapCompletedEvent {
            id: offer.trade_id,
            buyer: ctx.accounts.buyer_sol.key(),
            seller: ctx.accounts.external_seller_sol.key(),
            token_a_transferred: offer.token_a_offered_amount,
            token_b_transferred: offer.token_b_wanted_amount,
            is_swap_completed: true,
        });

        Ok(())
    }

    pub fn finalize_interchain_origin_sol_offer(
        ctx: Context<TakeInterchainOriginSolOffer>,
        id: u64,
    ) -> Result<()> {
        let offer = &mut ctx.accounts.interchain_origin_sol_offer;

        // Ensure the offer has not already been filled.
        require!(!offer.is_swap_completed, P2PError::SwapAlreadyCompleted);
        // Prevent a maker from filling their own offer.
        require!(
            offer.external_buyer_sol != ctx.accounts.seller_sol.key(),
            P2PError::MakerAndTakerCannotBeSame
        );

        msg!(
            "Finalizing cross-chain swap: {} USDT from vault to buyer {}",
            offer.token_a_offered_amount,
            ctx.accounts.external_buyer_sol.key()
        );

        // -------------------------------
        // Step 1: Transfer Buyer's asset from vault to seller. for origin is EVM chain
        // step 1: transfer seller's asset from vault to buyer for origin is SOL chain => external_seller_sol = buyer_sol and external_seller_sol_token_account_a = buyer_sol_token_account_a

        if offer.is_native {
            let vault_bump = ctx.bumps.vault_native;
            let seeds: &[&[u8]] = &[b"vault-native", &[vault_bump]];
            let signer_seeds = &[&seeds[..]];

            // -------------------------------
            let transfer_ix = anchor_lang::solana_program::system_instruction::transfer(
                &ctx.accounts.vault_native.key(),
                &ctx.accounts.external_buyer_sol.key(), //external_seller_sol = buyer_sol
                offer.token_a_offered_amount,
            );

            anchor_lang::solana_program::program::invoke_signed(
                &transfer_ix,
                &[
                    ctx.accounts.vault_native.to_account_info(),
                    ctx.accounts.external_buyer_sol.to_account_info(), //external_seller_sol = buyer_sol
                    ctx.accounts.system_program.to_account_info(),
                ],
                signer_seeds,
            )?;

            msg!(
                "Native SOL transferred {} lamports from native vault to buyer : {}.",
                offer.token_a_offered_amount,
                offer.external_buyer_sol
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
                        to: ctx
                            .accounts
                            .external_buyer_sol_token_account_a //external_seller_sol_token_account_a = buyer_sol_token_account_a
                            .to_account_info(),
                        authority: ctx.accounts.global_authority.to_account_info(),
                    },
                    &signer_seeds,
                ),
                offer.token_a_offered_amount,
            )?;

            msg!(
                "SPL tokens : {} transferred from vault to taker: {} tokens",
                offer.token_a_offered_amount,
                ctx.accounts.external_buyer_sol.key()
            );
        }

        // if you don't want on-chain state tracking, you can remove this line and just emit the event and close the offer account.
        offer.is_swap_completed = true;

        // emit logs
        // TODO : need to update this and event
        msg!(
            "Interchain origin sol offer completed for trade id: {},buyer : {} , seller_sol: {},
        seller_evm : {:?}, to buyer's sol address : {}, is_swap_completed: {}, token_a_offered_amount: {},token_b_wanted_amount: {}, is_taker_native: {}",
            offer.trade_id,
            offer.external_buyer_sol,
            offer.seller_sol,
            offer.seller_evm,
            offer.external_buyer_sol,
            offer.is_swap_completed,
            offer.token_a_offered_amount,
            offer.token_b_wanted_amount,
            offer.is_taker_native
        );

        emit!(InterchainSwapCompletedEvent {
            id: offer.trade_id,
            buyer: ctx.accounts.external_buyer_sol.key(),
            seller: ctx.accounts.seller_sol.key(),
            token_a_transferred: offer.token_a_offered_amount,
            token_b_transferred: offer.token_b_wanted_amount,
            is_swap_completed: true,
        });

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(id: u64)]
pub struct RelayOfferClone<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account()]
    pub token_mint_a: Account<'info, Mint>,

    #[account(
        init,
        payer = maker,
        // space: big enough for all fields
        space = 8 + InterchainOffer::SIZE,
        seeds = [b"InterChainoffer", maker.key().as_ref(), id.to_le_bytes().as_ref()],
        bump
    )]
    pub interchain_offer: Account<'info, InterchainOffer>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]
#[instruction(id: u64)]
pub struct TakeInterchainOriginSolOffer<'info> {
    #[account(mut)]
    pub seller_sol: Signer<'info>, // seller origin on sol chain, who deposited the asset into contract

    #[account(mut)]
    pub external_buyer_sol: SystemAccount<'info>, // Buyer : UserB's solana address who is taking the offer

    #[account()]
    pub token_mint_a: Account<'info, Mint>,

    // #[account()]
    // pub token_mint_b: Account<'info, Mint>,
    #[account(
        mut,
        close = seller_sol,  // This tells Anchor to close the offer account and send its lamports to the maker.
        seeds = [b"InterChainoffer", seller_sol.key().as_ref(), id.to_le_bytes().as_ref()],
        bump
    )]
    pub interchain_origin_sol_offer: Account<'info, InterchainOriginSOlOffer>,
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
        associated_token::authority = external_buyer_sol,
    )]
    pub external_buyer_sol_token_account_a: Account<'info, TokenAccount>, // token account of seller on sol chain where he received the asset

    // #[account(
    //     mut,
    //     associated_token::mint = token_mint_b,
    //     associated_token::authority = external_seller_sol,
    // )]
    // pub taker_token_account_b: Account<'info, TokenAccount>,

    // #[account(
    //     mut,
    //     token::mint = token_mint_b,
    //     token::authority = maker
    // )]
    // pub maker_token_account_b: Account<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]
#[instruction(id: u64)]
pub struct TakeInterchainOffer<'info> {
    #[account(mut)]
    pub external_seller_sol: Signer<'info>, // Seller : UserA's solana address

    #[account(mut)]
    pub buyer_sol: SystemAccount<'info>, // Buyer : UserB's solana address

    #[account()]
    pub token_mint_a: Account<'info, Mint>,

    // #[account()]
    // pub token_mint_b: Account<'info, Mint>,
    #[account(
        mut,
        close = external_seller_sol,  // This tells Anchor to close the offer account and send its lamports to the maker.
        seeds = [b"InterChainoffer", external_seller_sol.key().as_ref(), id.to_le_bytes().as_ref()],
        bump
    )]
    pub offer: Account<'info, InterchainOffer>,
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
        associated_token::authority = external_seller_sol,
    )]
    pub external_seller_sol_token_account_a: Account<'info, TokenAccount>,

    // #[account(
    //     mut,
    //     associated_token::mint = token_mint_b,
    //     associated_token::authority = external_seller_sol,
    // )]
    // pub taker_token_account_b: Account<'info, TokenAccount>,

    // #[account(
    //     mut,
    //     token::mint = token_mint_b,
    //     token::authority = maker
    // )]
    // pub maker_token_account_b: Account<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
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

/// Context for depositing raw SOL
#[derive(Accounts)]
#[instruction(id: u64)]
pub struct InterchainOriginSolMakeOfferNative<'info> {
    /// Person who deposits the SOL
    #[account(mut)]
    pub seller_sol: Signer<'info>,

    /// Just pass in any valid mint for 'A' and 'B' if you like.  
    /// Even if `is_native`, we’re not transferring SPL here.  
    #[account()]
    pub token_mint_a: Account<'info, Mint>,

    // #[account()]
    // pub token_mint_b: Account<'info, Mint>,
    /// The Offer PDA storing trade details
    // #[account(
    //     init,
    //     payer = maker,
    //     // space: big enough for all fields
    //     space = 8 + Offer::SIZE,
    //     seeds = [b"offer", maker.key().as_ref(), id.to_le_bytes().as_ref()],
    //     bump
    // )]
    // pub offer: Account<'info, Offer>,

    // #[account(
    //     mut,
    //     // close = external_seller_sol,  // This tells Anchor to close the offer account and send its lamports to the maker.
    //     seeds = [b"InterChainoffer", external_seller_sol.key().as_ref(), id.to_le_bytes().as_ref()],
    //     bump
    // )]
    // pub offer: Account<'info, InterchainOffer>,

    #[account(
        init,
        payer = seller_sol,
        // space: big enough for all fields
        space = 8 + InterchainOriginSOlOffer::SIZE,
        seeds = [b"InterChainoffer", seller_sol.key().as_ref(), id.to_le_bytes().as_ref()],
        bump
    )]
    pub interchain_origin_sol_offer: Account<'info, InterchainOriginSOlOffer>,

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
        payer = seller_sol,
        space = 0,
        seeds = [b"vault-native"],
        bump,
        owner = system_program::ID
    )]
    pub vault: AccountInfo<'info>, // Program-owned account

    pub system_program: Program<'info, System>,
}

/// Context for depositing raw SOL
#[derive(Accounts)]
#[instruction(id: u64, external_seller_sol: Pubkey)]
pub struct InterchainMakeOfferNative<'info> {
    /// Person who deposits the SOL
    #[account(mut)]
    pub buyer_sol: Signer<'info>,

    /// Just pass in any valid mint for 'A' and 'B' if you like.  
    /// Even if `is_native`, we’re not transferring SPL here.  
    #[account()]
    pub token_mint_a: Account<'info, Mint>,

    #[account()]
    pub token_mint_b: Account<'info, Mint>,

    /// The Offer PDA storing trade details
    // #[account(
    //     init,
    //     payer = maker,
    //     // space: big enough for all fields
    //     space = 8 + Offer::SIZE,
    //     seeds = [b"offer", maker.key().as_ref(), id.to_le_bytes().as_ref()],
    //     bump
    // )]
    // pub offer: Account<'info, Offer>,

    #[account(
        mut,
        // close = external_seller_sol,  // This tells Anchor to close the offer account and send its lamports to the maker.
        seeds = [b"InterChainoffer", external_seller_sol.key().as_ref(), id.to_le_bytes().as_ref()],
        bump
    )]
    pub offer: Account<'info, InterchainOffer>,

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
        payer = buyer_sol,
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

#[derive(Accounts)]
#[instruction(id: u64)]
pub struct InterchainOriginSolMakeOfferSpl<'info> {
    /// Person who deposits the SPL tokens
    #[account(mut)]
    pub seller_sol: Signer<'info>,

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
        token::authority = seller_sol
    )]
    pub seller_sol_token_account_a: Account<'info, TokenAccount>,

    /// The Offer PDA storing trade details
    // #[account(
    //     init,
    //     payer = maker,
    //     // space: big enough for all fields
    //     space = 8 + Offer::SIZE,
    //     seeds = [b"offer", maker.key().as_ref(), id.to_le_bytes().as_ref()],
    //     bump
    // )]
    // pub offer: Account<'info, Offer>,

    #[account(
        init,
        payer = seller_sol,
        // space: big enough for all fields
        space = 8 + InterchainOriginSOlOffer::SIZE,
        seeds = [b"InterChainoffer", seller_sol.key().as_ref(), id.to_le_bytes().as_ref()],
        bump
    )]
    pub interchain_origin_sol_offer: Account<'info, InterchainOriginSOlOffer>,

    #[account(
        init_if_needed,
        payer = seller_sol,
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

#[derive(Accounts)]
#[instruction(id: u64, external_seller_sol: Pubkey)]
pub struct InterchainMakeOfferSpl<'info> {
    /// Person who deposits the SPL tokens
    #[account(mut)]
    pub buyer_sol: Signer<'info>,

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
        token::authority = buyer_sol
    )]
    pub buyer_sol_token_account_a: Account<'info, TokenAccount>,

    /// The Offer PDA storing trade details
    // #[account(
    //     init,
    //     payer = maker,
    //     // space: big enough for all fields
    //     space = 8 + Offer::SIZE,
    //     seeds = [b"offer", maker.key().as_ref(), id.to_le_bytes().as_ref()],
    //     bump
    // )]
    // pub offer: Account<'info, Offer>,

    #[account(
        mut,
        // close = external_seller_sol,  // This tells Anchor to close the offer account and send its lamports to the maker.
        seeds = [b"InterChainoffer", external_seller_sol.key().as_ref(), id.to_le_bytes().as_ref()],
        bump
    )]
    pub offer: Account<'info, InterchainOffer>,

    #[account(
        init_if_needed,
        payer = buyer_sol,
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

#[account]
pub struct InterchainOriginSOlOffer {
    //seller origin deposit fields
    pub seller_sol: Pubkey,   // buyer : maker address on solana chain
    pub seller_evm: [u8; 20], // EVM address of buyer

    //relay fields
    pub trade_id: u64,                // same as evm trade id
    pub external_buyer_sol: Pubkey, // solana address of seller where he wants to receive the token
    pub external_buyer_evm: [u8; 20], // EVM address of seller
    pub is_seller_origin_sol: bool, // NO, seller origin is EVM
    pub is_taker_native: bool,      // NO, taker wants spl token
    pub is_swap_completed: bool,    // NO, swap is not completed
    pub is_native: bool,            // NO, seller is not offering native token
    pub chain_id: u64,

    // deposit amount fields
    pub token_a_offered_amount: u64, // seller offering 0.17 eth on evm chain
    pub token_b_wanted_amount: u64,  // seller wants 15 USDC on solana chain

    // buyer is transfering 15 USDC to seller which is spl-token
    // we need token mint account address for USDC spl token
    pub token_mint_a: Pubkey, // USDC mint account address
    //pub buyer: Option<Pubkey>, // buyer address on solana chain
    //pub buyer_token_account: Option<Pubkey>, // buyer token account address on solana chain
    pub fee_collected: u64, // fee collected by the relayer

    pub bump: u8, // bump for the account
}

impl InterchainOriginSOlOffer {
    pub const SIZE: usize = 32                    // buyer_sol
        + 20                    // buyer_evm
        + 32                    // trade_id
        + 32                    // maker
        + 20                    // external_seller
        + 1                     // is_seller_origin_sol
        + 1                     // is_taker_native
        + 1                     // is_swap_completed
        + 1                     // is_native
        + 8                     // chain_id
        + 8                     // token_a_offered_amount
        + 8                     // token_b_wanted_amount
        + 32                    // token_mint_a
        + 8; // fee_collected
}

#[account]
pub struct InterchainOffer {
    //buyer deposit fields
    pub buyer_sol: Pubkey,   // buyer : maker address on solana chain
    pub buyer_evm: [u8; 20], // EVM address of buyer

    //relay fields
    pub trade_id: u64,                 // same as evm trade id
    pub external_seller_sol: Pubkey, // solana address of seller where he wants to receive the token
    pub external_seller_evm: [u8; 20], // EVM address of seller
    pub is_seller_origin_sol: bool,  // NO, seller origin is EVM
    pub is_taker_native: bool,       // NO, taker wants spl token
    pub is_swap_completed: bool,     // NO, swap is not completed
    pub is_native: bool,             // NO, seller is not offering native token
    pub chain_id: u64,

    // deposit amount fields
    pub token_a_offered_amount: u64, // seller offering 0.17 eth on evm chain
    pub token_b_wanted_amount: u64,  // seller wants 15 USDC on solana chain

    // buyer is transfering 15 USDC to seller which is spl-token
    // we need token mint account address for USDC spl token
    pub token_mint_a: Pubkey, // USDC mint account address
    //pub buyer: Option<Pubkey>, // buyer address on solana chain
    //pub buyer_token_account: Option<Pubkey>, // buyer token account address on solana chain
    pub fee_collected: u64, // fee collected by the relayer

    pub bump: u8, // bump for the account
}

impl InterchainOffer {
    pub const SIZE: usize = 32                    // buyer_sol
        + 20                    // buyer_evm
        + 32                    // trade_id
        + 32                    // maker
        + 20                    // external_seller
        + 1                     // is_seller_origin_sol
        + 1                     // is_taker_native
        + 1                     // is_swap_completed
        + 1                     // is_native
        + 8                     // chain_id
        + 8                     // token_a_offered_amount
        + 8                     // token_b_wanted_amount
        + 32                    // token_mint_a
        + 8; // fee_collected
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

#[event]
pub struct InterchainOriginSolCreateTradeEvent {
    #[index]
    pub id: u64,
    pub seller_sol: Pubkey,
    pub seller_evm: [u8; 20],
    pub token_a_offered_amount: u64,
    pub token_b_wanted_amount: u64,
    pub is_taker_native: bool,
    pub is_native: bool,
    is_swap_completed: bool,
}

#[event]
pub struct InterchainCreateTradeEvent {
    #[index]
    pub id: u64,
    pub buyer: Pubkey,
    pub token_a_offered_amount: u64,
    pub token_b_wanted_amount: u64,
    pub is_taker_native: bool,
    is_swap_completed: bool,
}

/// Event emitted when a trade is completed.
#[event]
pub struct InterchainSwapCompletedEvent {
    #[index]
    pub id: u64,
    pub buyer: Pubkey,
    pub seller: Pubkey,
    pub token_a_transferred: u64,
    pub token_b_transferred: u64,
    pub is_swap_completed: bool,
}

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

#[event]
pub struct RelayEvmTradeEvent {
    pub trade_id: u64,
    pub external_seller_sol: Pubkey,
    pub external_seller_evm: [u8; 20],
    pub is_seller_origin_sol: bool,
    pub is_taker_native: bool,
    pub is_swap_completed: bool,
    pub is_native: bool,
    pub chain_id: u64,
    pub token_a_offered_amount: u64,
    pub token_b_wanted_amount: u64,
    pub token_mint_a: Pubkey,
    pub fee_collected: u64,
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
