import * as anchor from "@coral-xyz/anchor";
import { BN, Program } from "@coral-xyz/anchor";
import { ASSOCIATED_TOKEN_PROGRAM_ID, AuthorityType, getAssociatedTokenAddressSync, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { Keypair, LAMPORTS_PER_SOL, PublicKey, SystemProgram } from "@solana/web3.js";
import { assert } from "chai";
import crypto from "crypto";

import { Swap } from "../target/types/swap";

// User A (swap.json) and User B (chaidex.json) keypairs
const userA = Keypair.fromSecretKey(new Uint8Array(require("../swap.json")));
const userB = Keypair.fromSecretKey(new Uint8Array(require("../chaidex.json")));

// Replace with actual CT Token Mint address
const tokenMintA = new PublicKey("J1q7FEiMhzgd1T9bGtdh8ZTZa8mhsyszaW4AqQPvYxWX");
const tokenMintB = new PublicKey("J1q7FEiMhzgd1T9bGtdh8ZTZa8mhsyszaW4AqQPvYxWX");



describe.skip("swap-maker", () => {
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);
    const program = anchor.workspace.Swap as Program<Swap>;

    console.log("Program ID:", program.programId.toString());

    // const accounts: any = {

    // };

    const randomSeed = crypto.randomBytes(4).readUInt32LE(0);
    const offerId = new BN(randomSeed);

    let offerPda: PublicKey;
    let vaultTokenAccount: PublicKey;
    let userATokenAccount: PublicKey;
    let userBTokenAccount: PublicKey;

    it("Deposit raw SOL (native)", async () => {
        console.log(`User A: ${userA.publicKey.toBase58()}`);
        console.log(`User B: ${userB.publicKey.toBase58()}`);

        // 🛠 **Fix Offer PDA Calculation**
        // const idBuffer = Buffer.alloc(8);
        // idBuffer.writeBigUInt64LE(BigInt(offerId.toString()));

        // [offerPda] = PublicKey.findProgramAddressSync(
        //     [Buffer.from("offer"), userA.publicKey.toBuffer(), idBuffer],
        //     program.programId
        // );

        console.log(`Using Offer ID: ${offerId.toString()}`); // ✅ Log Offer ID

        // Fix the endianness issue
        const idLE = new BN(offerId).toArrayLike(Buffer, "le", 8);

        [offerPda] = PublicKey.findProgramAddressSync(
            [
                Buffer.from("offer"),
                userA.publicKey.toBuffer(),
                idLE,
            ],
            program.programId
        );

        const [vaultPda] = PublicKey.findProgramAddressSync(
            [
                Buffer.from("vault-native")
                // userA.publicKey.toBuffer(),
                // idLE,
            ],
            program.programId
        );

        console.log("offerPda =", offerPda.toBase58());
        console.log("vaultPda =", vaultPda.toBase58());

        // userATokenAccount = getAssociatedTokenAddressSync(tokenMintA, userA.publicKey);
        // userBTokenAccount = getAssociatedTokenAddressSync(tokenMintA, userB.publicKey);

        // save accounts for later use
        // accounts.maker = userA.publicKey;
        // accounts.taker = userB.publicKey;
        // accounts.tokenMintA = tokenMintA;
        // accounts.tokenMintB = tokenMintA;
        // accounts.makerTokenAccountA = userATokenAccount;
        // accounts.takerTokenAccountB = userBTokenAccount;
        // accounts.offer = offerPda;
        // accounts.vault = vaultTokenAccount;
        // accounts.systemProgram = SystemProgram.programId;
        // accounts.associatedTokenProgram = ASSOCIATED_TOKEN_PROGRAM_ID;
        // accounts.tokenProgram = TOKEN_PROGRAM_ID;

        // try {
        //     const userABalance = await provider.connection.getTokenAccountBalance(userATokenAccount);
        //     console.log(`User A CT Balance: ${userABalance.value.uiAmount} CT`);

        //     const userBBalance = await provider.connection.getTokenAccountBalance(userBTokenAccount);
        //     console.log(`User B CT Balance: ${userBBalance.value.uiAmount} CT`);

        //     const vaultBalance = await provider.connection.getTokenAccountBalance(vaultTokenAccount);
        //     console.log(`Vault CT Balance: ${vaultBalance.value.uiAmount} CT`);
        // } catch (error) {
        //     console.warn("Error fetching token balances. Maybe the accounts don’t exist yet.");
        // }
        const vaultBalanceBefore = await provider.connection.getBalance(vaultPda);
        console.log("Vault lamport balance before:", vaultBalanceBefore);



        // 🚀 **Step 2: Call deposit_seller_native**
        const tx = await program.methods
            .depositSellerNative(

                offerId, // Trade ID
                new BN(15000000000), // Token B wanted amount (15 CT)
                new BN(100000000), // Token A (SOL) offered amount
                false, // is_taker_native

            )
            .accounts({
                maker: userA.publicKey,
                tokenMintA: tokenMintA,
                tokenMintB: tokenMintA,
                offer: offerPda,
                vault: vaultPda,
                systemProgram: SystemProgram.programId,
            })
            .signers([userA])
            .rpc();

        console.log("✅ Transaction successful! Signature:", tx);

        const vaultBalance = await provider.connection.getBalance(vaultPda);
        console.log("Vault lamport balance after:", vaultBalance);

        // Fetch offer data
        const offerAccount = await program.account.offer.fetch(offerPda);
        assert.ok(offerAccount.id.eq(offerId), " Offer ID does not match expected value.");
        assert.equal(offerAccount.maker.toBase58(), userA.publicKey.toBase58(), " Maker does not match expected value.");
        assert.equal(offerAccount.isNative, true, " isNative does not match expected value.");
        assert.equal(offerAccount.isTakerNative, false, " isTakerNative does not match expected value.");
        assert.equal(offerAccount.isSwapCompleted, false, " isSwapCompleted does not match expected value.");
        console.log("Offer stored for native deposit, id =", offerAccount.id.toString());
    });

    it("Deposit SPL Tokens (non-native)", async () => {
        console.log("\n--- Now testing deposit_seller_spl ---");

        const randomSeedSpl = crypto.randomBytes(4).readUInt32LE(0);
        const offerIdSpl = new BN(randomSeedSpl);
        console.log("Using SPL Offer ID:", offerIdSpl.toString());

        // Derive PDAs
        const idLEspl = offerIdSpl.toArrayLike(Buffer, "le", 8);

        // We assume userA has a token account with at least 600 CT
        const makerTokenAccountA = getAssociatedTokenAddressSync(
            tokenMintA,
            userA.publicKey,
            false,
            TOKEN_PROGRAM_ID,
            ASSOCIATED_TOKEN_PROGRAM_ID
        );

        // Check userA’s balance before deposit
        const makerTokenAccInfoBefore = await provider.connection.getTokenAccountBalance(makerTokenAccountA);
        console.log(`User A CT token balance before deposit: ${makerTokenAccInfoBefore.value.uiAmount} CT`);


        const [offerPdaSpl] = PublicKey.findProgramAddressSync(
            [Buffer.from("offer"), userA.publicKey.toBuffer(), idLEspl],
            program.programId
        );

        // Derive the global authority PDA
        const [globalAuthorityPda] = PublicKey.findProgramAddressSync(
            [Buffer.from("global-authority")], // Fixed seed
            program.programId
        );
        const vaultSplAta = getAssociatedTokenAddressSync(
            tokenMintA,
            globalAuthorityPda,
            true, // allowOwnerOffCurve = true
            TOKEN_PROGRAM_ID,
            ASSOCIATED_TOKEN_PROGRAM_ID
        );

        console.log("Offer SPL PDA =", offerPdaSpl.toBase58());
        console.log("Vault SPL ATA =", vaultSplAta.toBase58());

        const tokenAOfferedAmount = new BN(11000000000); // 11 tokens
        const tokenBWantedAmount = new BN(100000000) // 0.1 sol

        // spl_vault balance before deposit
        //const vaultBalanceBeforeSpl = await provider.connection.getTokenAccountBalance(vaultSplAta);
        //console.log("Vault CT token balance before deposit:", vaultBalanceBeforeSpl.value.uiAmount);

        const txSPL = await program.methods.depositSellerSpl(
            offerIdSpl,
            tokenBWantedAmount,
            tokenAOfferedAmount,
            true // is_taker_native
        ).accounts({
            maker: userA.publicKey,
            tokenMintA: tokenMintA,
            tokenMintB: tokenMintA,
            makerTokenAccountA: makerTokenAccountA,
            offer: offerPdaSpl,
            vault_spl: vaultSplAta,
            globalAuthority: globalAuthorityPda,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        }).signers([userA]).rpc();

        console.log("SPL deposit txn signature:", txSPL);

        // spl_vault balance after deposit
        const vaultBalanceAfterSpl = await provider.connection.getTokenAccountBalance(vaultSplAta);
        console.log("Vault CT token balance after deposit:", vaultBalanceAfterSpl.value.uiAmount);

        // Check userA’s balance after deposit
        const makerTokenAccInfoAfter = await provider.connection.getTokenAccountBalance(makerTokenAccountA);
        console.log(`User A CT token balance after deposit: ${makerTokenAccInfoAfter.value.uiAmount} CT`);

        // Check vault balance
        const vaultBalance = await provider.connection.getTokenAccountBalance(vaultSplAta);
        console.log(`Vault CT token balance: ${vaultBalance.value.uiAmount} CT`);

        // assert.equal(
        //     vaultBalance.value.uiAmount, tokenAOfferedAmount.toNumber(),
        //     "Vault balance does not match expected deposit."
        // );

        //fetch the offer data
        // Fetch the offer data
        const offerAccountSpl = await program.account.offer.fetch(offerPdaSpl);
        assert.ok(offerAccountSpl.id.eq(offerIdSpl));
        assert.equal(offerAccountSpl.maker.toBase58(), userA.publicKey.toBase58());
        assert.equal(offerAccountSpl.isNative, false);
        assert.equal(offerAccountSpl.isTakerNative, true);
        assert.equal(offerAccountSpl.isSwapCompleted, false);
        console.log(
            "Offer stored for SPL deposit, id =",
            offerAccountSpl.id.toString()
        );

    });

});

describe("swap-taker", () => {

    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);
    const program = anchor.workspace.Swap as Program<Swap>;

    console.log("Program ID:", program.programId.toString());

    it("take offer details test", async () => {

        const offerAccount = await program.account.offer.fetch("HWa7yj4sAuVJ4F9nHiVUKz5Q9eQ6yq7xfpogwXu8t9u6");
        let offerId = offerAccount.id;
        let isNative = offerAccount.isNative;
        let isTakerNative = offerAccount.isTakerNative;
        let tokenAOfferedAmount = offerAccount.tokenAOfferedAmount;
        let tokenBWantedAmount = offerAccount.tokenBWantedAmount;


        console.log("Offer ID:", offerId.toString());
        console.log("isNative:", isNative);
        console.log("isTakerNative:", isTakerNative);
        console.log("Token A offered amount:", tokenAOfferedAmount.toString());
        console.log("Token B wanted amount:", tokenBWantedAmount.toString());
        console.log("Maker:", offerAccount.maker.toBase58());


    });


    it("take offer details test-------------------------", async () => {

        //const randomSeed = crypto.randomBytes(4).readUInt32LE(0);
        const offerId = new BN(1420099893);
        console.log("Using Offer ID:", offerId.toString());

        const idLE = offerId.toArrayLike(Buffer, "le", 8);

        // compute the offer PDA
        const [offerPda] = PublicKey.findProgramAddressSync(
            [Buffer.from("offer"), userA.publicKey.toBuffer(), idLE],
            program.programId
        )

        console.log("Offer PDA:", offerPda.toBase58());

        // Compute the global native vault PDA (seed: "vault-native")
        const [vaultNativePda] = PublicKey.findProgramAddressSync(
            [Buffer.from("vault-native")],
            program.programId
        );
        // Compute the global authority PDA (seed: "global-authority")
        const [globalAuthorityPda] = PublicKey.findProgramAddressSync(
            [Buffer.from("global-authority")],
            program.programId
        );

        // Compute the SPL vault ATA (even though not used for native deposit, it must be provided)
        const vaultSplAta = getAssociatedTokenAddressSync(
            tokenMintA,
            globalAuthorityPda,
            true,
            TOKEN_PROGRAM_ID,
            ASSOCIATED_TOKEN_PROGRAM_ID
        );

        console.log("Vault Native PDA:", vaultNativePda.toBase58());
        console.log("Vault SPL ATA ", vaultSplAta.toBase58());


        // Taker's associated token account for token mint A (for receiving maker’s SPL deposit if applicable)
        const takerTokenAccountA = getAssociatedTokenAddressSync(
            tokenMintA,
            userB.publicKey,
            false,
            TOKEN_PROGRAM_ID,
            ASSOCIATED_TOKEN_PROGRAM_ID

        );

        // Taker's token account for sending asset to maker (since offer.isTakerNative is false in this native deposit scenario)
        const takerTokenAccountB = getAssociatedTokenAddressSync(
            tokenMintA,
            userB.publicKey,
            false,
            TOKEN_PROGRAM_ID,
            ASSOCIATED_TOKEN_PROGRAM_ID
        );
        // Maker's token account for receiving taker's asset.
        const makerTokenAccountB = getAssociatedTokenAddressSync(
            tokenMintB,
            userA.publicKey,
            false,
            TOKEN_PROGRAM_ID,
            ASSOCIATED_TOKEN_PROGRAM_ID
        );

        const offerAccountBefore = await program.account.offer.fetch(offerPda);
        console.log("<-----Offer details before swap----->");
        console.log("  Offer ID:", offerAccountBefore.id.toString());
        console.log("  isNative:", offerAccountBefore.isNative);
        console.log("  isTakerNative:", offerAccountBefore.isTakerNative);
        console.log("  Token A offered amount:", offerAccountBefore.tokenAOfferedAmount.toString());
        console.log("  Token B wanted amount:", offerAccountBefore.tokenBWantedAmount.toString());
        console.log("  Maker:", offerAccountBefore.maker.toBase58());

        console.log(`transfering native sol : ${offerAccountBefore.tokenAOfferedAmount.toString()} from vault to userB and CT tokens : ${offerAccountBefore.tokenBWantedAmount.toString()} from userB to userA`);

        // check vault balance,UserB and UserA balance before swap
        const vaultBalanceBefore = await provider.connection.getBalance(vaultNativePda);
        console.log("Vault balance before swap:", vaultBalanceBefore);

        const userBBalanceBefore = await provider.connection.getBalance(userB.publicKey);
        console.log("UserB balance before swap:", userBBalanceBefore);

        // userA (maker) tokenAccountB balance before swap
        const UserATokenAccB = await provider.connection.getTokenAccountBalance(makerTokenAccountB);
        console.log("UserA token account B(CT Tokens) balance before swap:", UserATokenAccB.value.uiAmount);

        const tx = await program.methods.takeOffer(offerId).accounts(
            {

                taker: userB.publicKey,
                maker: userA.publicKey,
                tokenMintA: tokenMintA,
                tokenMintB: tokenMintB,
                offer: offerPda,
                vaultNative: vaultNativePda,
                vaultSpl: vaultSplAta,
                globalAuthority: globalAuthorityPda,
                takerTokenAccountA: takerTokenAccountA,
                takerTokenAccountB: takerTokenAccountB,
                makerTokenAccountB: makerTokenAccountB,
                tokenProgram: TOKEN_PROGRAM_ID,
                systemProgram: SystemProgram.programId,
                associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            }
        ).signers([userB]).rpc();

        console.log("Take offer transaction signature:", tx);

        // chek vault balance,UserB and UserA balance after swap
        const vaultBalanceAfter = await provider.connection.getBalance(vaultNativePda);
        console.log("Vault balance after swap:", vaultBalanceAfter);

        const userBBalanceAfter = await provider.connection.getBalance(userB.publicKey);
        console.log("UserB balance after swap:", userBBalanceAfter);

        // userA (maker) tokenAccountB balance after swap
        const UserATokenAccBAfter = await provider.connection.getTokenAccountBalance(makerTokenAccountB);
        console.log("UserA token account B(CT Tokens) balance after swap:", UserATokenAccBAfter.value.uiAmount);

        // Verify that the offer account has been closed.
        try {
            await program.account.offer.fetch(offerPda);
            assert.fail("Offer account should be closed after swap");
        } catch (err) {
            console.log("Offer account is closed as expected.");
        }

        // Check maker's token account B balance (should have increased by tokenBWantedAmount).
        const makerTokenAccB = await provider.connection.getTokenAccountBalance(makerTokenAccountB);
        console.log("Maker token account B balance after swap:", makerTokenAccB.value.uiAmount);

    });
});
