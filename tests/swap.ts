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

const offerCatch = new Map();

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
        // accounts.buyerSolTokenAccountA = userATokenAccount;
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

describe("interchain-swap-maker", () => {
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);
    const program = anchor.workspace.Swap as Program<Swap>;

    console.log("Program ID:", program.programId.toString());

    // let offerPda: PublicKey;
    let vaultTokenAccount: PublicKey;
    let userATokenAccount: PublicKey;
    let userBTokenAccount: PublicKey;

    it.skip("Deposit interchain raw SOL (native)", async () => {
        console.log(`User A: ${userA.publicKey.toBase58()}`);
        console.log(`User B: ${userB.publicKey.toBase58()}`);


        const randomSeed = crypto.randomBytes(4).readUInt32LE(0);
        const offerId = new BN(2946257162);

        console.log(`Using Offer ID: ${offerId.toString()}`); // ✅ Log Offer ID

        // Fix the endianness issue
        const idLE = new BN(offerId).toArrayLike(Buffer, "le", 8);
        //offerPda 
        let offerPda = new PublicKey("38gnfp3sK1pqu4kRcHdZrEb97aneupqcNpHzarwqC7kS")

        const offerAccount = await program.account.interchainOffer.fetch("38gnfp3sK1pqu4kRcHdZrEb97aneupqcNpHzarwqC7kS");
        console.log("Offer data:", offerAccount);

        if (offerId.eq(offerAccount.tradeId)) {
            console.log("Offer ID matches");
        } else {
            console.log("Offer ID does not match");
            return;
        }

        const [vaultPda] = PublicKey.findProgramAddressSync(
            [
                Buffer.from("vault-native")
                // userA.publicKey.toBuffer(),
                // idLE,
            ],
            program.programId
        );

        console.log("offerPda =", offerPda);
        console.log("vaultPda =", vaultPda.toBase58());

        const vaultBalanceBefore = await provider.connection.getBalance(vaultPda);
        console.log("Vault lamport balance before:", vaultBalanceBefore);



        // 🚀 **Step 2: Call deposit_seller_native**
        const tx = await program.methods
            .interchainDepositSellerNative(

                offerAccount.tradeId,
                offerAccount.externalSellerSol,
                offerAccount.externalSellerEvm,
                offerAccount.tokenAOfferedAmount,
                offerAccount.tokenBWantedAmount,
                offerAccount.isTakerNative,

            )
            .accounts({
                buyerSol: userA.publicKey,
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
        // const offerAccount = await program.account.interchainOffer.fetch(offerPda);
        //console.log("Offer data:", offerAccount);
        //     assert.ok(offerAccount.id.eq(offerId), " Offer ID does not match expected value.");
        //     assert.equal(offerAccount.maker.toBase58(), userA.publicKey.toBase58(), " Maker does not match expected value.");
        //     assert.equal(offerAccount.isNative, true, " isNative does not match expected value.");
        //     assert.equal(offerAccount.isTakerNative, false, " isTakerNative does not match expected value.");
        //     assert.equal(offerAccount.isSwapCompleted, false, " isSwapCompleted does not match expected value.");
        //     console.log("Offer stored for native deposit, id =", offerAccount.id.toString());
    });

    it.skip("log offer details before tx", async () => {
        const offerAccount = await program.account.interchainOffer.fetch("J7XPWeUMVM12fXbhAsxSDF5xajzoVVsC2S5fW5cdJtES");
        console.log("Offer data after interchain deposit ------>", offerAccount);
    });

    it("Deposit Interchain SPL Tokens (non-native)", async () => {
        console.log("\n--- Now testing deposit_seller_spl ---");

        const randomSeedSpl = crypto.randomBytes(4).readUInt32LE(0);
        const offerIdSpl = new BN(1709950475);
        console.log("Using SPL Offer ID:", offerIdSpl.toString());

        // Derive PDAs
        const idLEspl = offerIdSpl.toArrayLike(Buffer, "le", 8);

        let offerPdaSpl = new PublicKey("J7XPWeUMVM12fXbhAsxSDF5xajzoVVsC2S5fW5cdJtES")

        const offerAccount = await program.account.interchainOffer.fetch("J7XPWeUMVM12fXbhAsxSDF5xajzoVVsC2S5fW5cdJtES");
        console.log("offer data details before deposit--------->", offerAccount);

        if (offerIdSpl.eq(offerAccount.tradeId)) {
            console.log("Offer ID matches");
        } else {
            console.log("Offer ID does not match");
            return;
        }

        if (userA.publicKey === offerAccount.buyerSol) {
            console.log("Buyer matches");
        } else {
            console.log("Buyer does not match");
            console.log("Buyer:", userA.publicKey.toBase58());
            console.log("Offer Buyer:", offerAccount.buyerSol.toBase58());
        }

        // We assume userA has a token account with at least 600 CT
        const buyerSolTokenAccountA = getAssociatedTokenAddressSync(
            tokenMintA,
            userA.publicKey,
            false,
            TOKEN_PROGRAM_ID,
            ASSOCIATED_TOKEN_PROGRAM_ID
        );

        // Check userA’s balance before deposit
        // const makerTokenAccInfoBefore = await provider.connection.getTokenAccountBalance(buyerSolTokenAccountA);
        // console.log(`User A CT token balance before deposit: ${makerTokenAccInfoBefore.value.uiAmount} CT`);


        // const [offerPdaSpl] = PublicKey.findProgramAddressSync(
        //     [Buffer.from("offer"), userA.publicKey.toBuffer(), idLEspl],
        //     program.programId
        // );

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

        //console.log("Offer SPL PDA =", offerPdaSpl);
        console.log("Vault SPL ATA =", vaultSplAta.toBase58());

        // const tokenAOfferedAmount = new BN(11000000000); // 11 tokens
        // const tokenBWantedAmount = new BN(100000000) // 0.1 sol

        // spl_vault balance before deposit
        //const vaultBalanceBeforeSpl = await provider.connection.getTokenAccountBalance(vaultSplAta);
        //console.log("Vault CT token balance before deposit:", vaultBalanceBeforeSpl.value.uiAmount);

        const txSPL = await program.methods.interchainDepositSellerSpl(
            offerAccount.tradeId,
            offerAccount.externalSellerSol,
            offerAccount.externalSellerEvm,
            offerAccount.tokenAOfferedAmount,
            offerAccount.tokenBWantedAmount,
            offerAccount.isTakerNative,
        ).accounts({
            buyerSol: userA.publicKey,
            tokenMintA: tokenMintA,
            tokenMintB: tokenMintA,
            buyerSolTokenAccountA: buyerSolTokenAccountA,
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
        const makerTokenAccInfoAfter = await provider.connection.getTokenAccountBalance(buyerSolTokenAccountA);
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
        const offerAccountSpl = await program.account.interchainOffer.fetch("J7XPWeUMVM12fXbhAsxSDF5xajzoVVsC2S5fW5cdJtES");
        console.log("Offer data after interchain deposit ------>", offerAccountSpl);
        assert.ok(offerAccountSpl.tradeId.eq(offerIdSpl));
        assert.equal(offerAccountSpl.buyerSol.toBase58(), userA.publicKey.toBase58());
        assert.equal(offerAccountSpl.isNative, false);
        assert.equal(offerAccountSpl.isTakerNative, false);
        assert.equal(offerAccountSpl.isSwapCompleted, false);
        console.log(
            "Offer stored for SPL deposit, id =",
            offerAccountSpl.tradeId.toString()
        );

    });

});


describe.skip("swap-taker", () => {

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

describe.skip("interchain-native-relay-data", () => {

    //1. set up the Anchor provider and program
    const provider = anchor.AnchorProvider.env();
    const connection = provider.connection;
    anchor.setProvider(provider);
    const program = anchor.workspace.Swap as Program<Swap>;

    const externalSellerSol = new PublicKey(
        "DYNnymGWfKKqYgwRuxYZq3f4qDtQ1LLaXogWhchHrjfQ"
    );

    const evmHexAddress = "c629Fa8B87AD97E92C448E56Df9d979E1D1f441f".toLowerCase();
    const evemAddressBytes = Buffer.from(evmHexAddress, "hex"); // 20 bytes

    let interchainOfferPda: PublicKey;


    it("relayer calls relay_offer_clone", async () => {
        // Step A: Generate random ID for the trade
        const randomSeed = crypto.randomBytes(4).readUInt32LE(0);
        const tradeId = new BN(randomSeed); // or a fixed number if you prefer
        console.log("Using Trade ID:", tradeId.toString());

        const tokenAOfferedAmount = new BN("170000000000000000"); // 0.17 ETH in wei
        const tokenBWantedAmount = new BN("50000000"); // 0.05 SOL (9 decimals)

        const chainId = new BN(1); // Ethereum mainnet
        const isTakerNative = false; // Let's say the buyer on Solana is paying with an SPL token, not SOL

        // Step B: Derive the PDA for the offer
        const idLE = tradeId.toArrayLike(Buffer, "le", 8);
        const [interchainOfferPdaPubkey, bump] = await PublicKey.findProgramAddressSync(
            [Buffer.from("InterChainoffer"),
            userB.publicKey.toBuffer(),
                idLE
            ],
            program.programId
        );
        interchainOfferPda = interchainOfferPdaPubkey;

        console.log("InterchainOffer PDA:", interchainOfferPda.toBase58());
        console.log("Bump found:", bump);

        const externalSellerEvm = Array.from(evemAddressBytes);



        // Step C: Call the relay_offer_clone method
        const txSig = await program.methods.relayOfferClone(
            tradeId,
            externalSellerEvm,
            externalSellerSol,
            tokenAOfferedAmount,
            tokenBWantedAmount,
            isTakerNative,
            chainId
        ).accounts({

            maker: userB.publicKey,
            tokenMintA: tokenMintA,
            interchainOffer: interchainOfferPda,
            systemProgram: SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,

        }).signers([userB]).rpc();

        console.log("relay_offer_clone tx signature:", txSig);


        // Step D: Fetch the offer data
        const offerAccount = await program.account.interchainOffer.fetch(interchainOfferPda);
        //console.log("offerAccount data:", offerAccount);

        // Step E: Log the EVM address from the offer account
        // We assume the field is named "externalSellerEvm" in your account.
        const rawEvmBytes = Buffer.from(offerAccount.externalSellerEvm);
        // Convert to a "0x" prefixed hex string
        const evmAddrHex = "0x" + rawEvmBytes.toString("hex");
        // If you'd rather uppercase it: 
        // const evmAddrHex = "0x" + rawEvmBytes.toString("hex").toUpperCase();

        console.log("EVM address from account:", evmAddrHex);

        //store the offer details in a map/DB
        offerCatch.set("tradeId", offerAccount.tradeId.toString());
        offerCatch.set("externalSellerEvm", evmAddrHex);
        offerCatch.set("externalSellerSol", offerAccount.externalSellerSol.toBase58());
        offerCatch.set("tokenAOfferedAmount", offerAccount.tokenAOfferedAmount.toString());
        offerCatch.set("tokenBWantedAmount", offerAccount.tokenBWantedAmount.toString());
        offerCatch.set("isTakerNative", offerAccount.isTakerNative.toString());
        offerCatch.set("chainId", offerAccount.chainId.toString());
        offerCatch.set("isSwapCompleted", offerAccount.isSwapCompleted.toString());
        offerCatch.set("isSellerOriginSol ", offerAccount.isSellerOriginSol.toString());
        offerCatch.set("feeCollected", offerAccount.feeCollected.toString());

        console.log("Offer details:", offerCatch);

        //offer details are fetched and stored now you are confirmed that 
        // relayer has successfully relayed the evm offer to solana
        // you can now show this all offers to the user and let them choose the offer they want to take

        // step F: Now the taker can take the offer by calling the deposit_native or deposit_spl method
        // and providing the same tradeId to the method
        // stored offer details that you stored while calling deposit_native or deposit_spl method
        // fetch the offer details from depsoit_native or deposit_spl method and compare with the stored offer details

        //step G: if the offer details has info like UserB has deposited 500 USDT on solana
        // then relayer can call the evm fn to send 0.17 ETH from evm contract to userB evm address
        // and emit the event BuyerWithdrawn

        //step H: relayer can see the BuyerWithdrawn event and then call the finalize_withdrawal method on solana to send 500 USDT from sol contract to UserA's sol address
        // and emit the event SellerWithdrawn
        // swap is completed now


    });


});

describe.skip("interchain-spl-relay-data", () => {

    //1. set up the Anchor provider and program
    const provider = anchor.AnchorProvider.env();
    const connection = provider.connection;
    anchor.setProvider(provider);
    const program = anchor.workspace.Swap as Program<Swap>;

    const externalSellerSol = new PublicKey(
        "DYNnymGWfKKqYgwRuxYZq3f4qDtQ1LLaXogWhchHrjfQ"
    );

    const evmHexAddress = "c629Fa8B87AD97E92C448E56Df9d979E1D1f441f".toLowerCase();
    const evemAddressBytes = Buffer.from(evmHexAddress, "hex"); // 20 bytes

    let interchainOfferPda: PublicKey;


    it("relayer calls relay_offer_clone", async () => {
        // Step A: Generate random ID for the trade
        const randomSeed = crypto.randomBytes(4).readUInt32LE(0);
        const tradeId = new BN(randomSeed); // or a fixed number if you prefer
        console.log("Using Trade ID:", tradeId.toString());

        const tokenAOfferedAmount = new BN("170000000000000000"); // 0.17 ETH in wei
        const tokenBWantedAmount = new BN("15000000000"); // 15 CT tokens (9 decimals)

        const chainId = new BN(1); // Ethereum mainnet
        const isTakerNative = false; // Let's say the buyer on Solana is paying with an SPL token, not SOL

        // Step B: Derive the PDA for the offer
        const idLE = tradeId.toArrayLike(Buffer, "le", 8);
        const [interchainOfferPdaPubkey, bump] = await PublicKey.findProgramAddressSync(
            [Buffer.from("InterChainoffer"),
            userB.publicKey.toBuffer(),
                idLE
            ],
            program.programId
        );
        interchainOfferPda = interchainOfferPdaPubkey;

        console.log("InterchainOffer PDA:", interchainOfferPda.toBase58());
        console.log("Bump found:", bump);

        const externalSellerEvm = Array.from(evemAddressBytes);



        // Step C: Call the relay_offer_clone method
        const txSig = await program.methods.relayOfferClone(
            tradeId,
            externalSellerEvm,
            externalSellerSol,
            tokenAOfferedAmount,
            tokenBWantedAmount,
            isTakerNative,
            chainId
        ).accounts({

            maker: userB.publicKey,
            tokenMintA: tokenMintA,
            interchainOffer: interchainOfferPda,
            systemProgram: SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,

        }).signers([userB]).rpc();

        console.log("relay_offer_clone tx signature:", txSig);


        // Step D: Fetch the offer data
        const offerAccount = await program.account.interchainOffer.fetch(interchainOfferPda);
        //console.log("offerAccount data:", offerAccount);

        // Step E: Log the EVM address from the offer account
        // We assume the field is named "externalSellerEvm" in your account.
        const rawEvmBytes = Buffer.from(offerAccount.externalSellerEvm);
        // Convert to a "0x" prefixed hex string
        const evmAddrHex = "0x" + rawEvmBytes.toString("hex");
        // If you'd rather uppercase it: 
        // const evmAddrHex = "0x" + rawEvmBytes.toString("hex").toUpperCase();

        console.log("EVM address from account:", evmAddrHex);

        //store the offer details in a map/DB
        offerCatch.set("tradeId", offerAccount.tradeId.toString());
        offerCatch.set("externalSellerEvm", evmAddrHex);
        offerCatch.set("externalSellerSol", offerAccount.externalSellerSol.toBase58());
        offerCatch.set("tokenAOfferedAmount", offerAccount.tokenAOfferedAmount.toString());
        offerCatch.set("tokenBWantedAmount", offerAccount.tokenBWantedAmount.toString());
        offerCatch.set("isTakerNative", offerAccount.isTakerNative.toString());
        offerCatch.set("chainId", offerAccount.chainId.toString());
        offerCatch.set("isSwapCompleted", offerAccount.isSwapCompleted.toString());
        offerCatch.set("isSellerOriginSol ", offerAccount.isSellerOriginSol.toString());
        offerCatch.set("feeCollected", offerAccount.feeCollected.toString());

        console.log("Offer details:", offerCatch);

        //offer details are fetched and stored now you are confirmed that 
        // relayer has successfully relayed the evm offer to solana
        // you can now show this all offers to the user and let them choose the offer they want to take

        // step F: Now the taker can take the offer by calling the deposit_native or deposit_spl method
        // and providing the same tradeId to the method
        // stored offer details that you stored while calling deposit_native or deposit_spl method
        // fetch the offer details from depsoit_native or deposit_spl method and compare with the stored offer details

        //step G: if the offer details has info like UserB has deposited 500 USDT on solana
        // then relayer can call the evm fn to send 0.17 ETH from evm contract to userB evm address
        // and emit the event BuyerWithdrawn

        //step H: relayer can see the BuyerWithdrawn event and then call the finalize_withdrawal method on solana to send 500 USDT from sol contract to UserA's sol address
        // and emit the event SellerWithdrawn
        // swap is completed now


    });


});


