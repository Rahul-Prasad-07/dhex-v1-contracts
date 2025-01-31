import * as anchor from "@coral-xyz/anchor";
import { BN, Program } from "@coral-xyz/anchor";
import { assert } from "chai";
import { Swap } from "../target/types/swap";
import { PublicKey, Keypair, SystemProgram, LAMPORTS_PER_SOL } from "@solana/web3.js";
import {
    TOKEN_PROGRAM_ID,
    getAssociatedTokenAddressSync,
    ASSOCIATED_TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import crypto from "crypto";

// User A (swap.json) and User B (chaidex.json) keypairs
const userA = Keypair.fromSecretKey(new Uint8Array(require("../swap.json")));
const userB = Keypair.fromSecretKey(new Uint8Array(require("../chaidex.json")));

// Replace with actual CT Token Mint address
const tokenMintA = new PublicKey("J1q7FEiMhzgd1T9bGtdh8ZTZa8mhsyszaW4AqQPvYxWX");




describe("swap", () => {
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
                Buffer.from("offer-native"),
                userA.publicKey.toBuffer(),
                idLE,
            ],
            program.programId
        );

        const [vaultPda] = PublicKey.findProgramAddressSync(
            [
                Buffer.from("vault-native"),
                userA.publicKey.toBuffer(),
                idLE,
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



        // 🚀 **Step 2: Call deposit_seller_native**
        const tx = await program.methods
            .depositSellerNative(

                offerId, // Trade ID
                new BN(15000000000), // Token B wanted amount (15 CT)
                new BN(500000000), // Token A (SOL) offered amount

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
        console.log("Vault lamport balance:", vaultBalance);

        assert(
            vaultBalance >= 500000000 - 10_000,
            "Vault balance not matching expected deposit."
        );

        // Fetch offer data
        const offerAccount = await program.account.offer.fetch(offerPda);
        assert.ok(offerAccount.id.eq(offerId));
        assert.equal(offerAccount.maker.toBase58(), userA.publicKey.toBase58());
        assert.equal(offerAccount.isNative, true);
        console.log("Offer stored for native deposit, id =", offerAccount.id.toString());
    });

    it("Deposit SPL Tokens (non-native)", async () => {
        console.log("\n--- Now testing deposit_seller_spl ---");

        const randomSeedSpl = crypto.randomBytes(4).readUInt32LE(0);
        const offerIdSpl = new BN(randomSeedSpl);
        console.log("Using SPL Offer ID:", offerIdSpl.toString());

        // Derive PDAs
        const idLEspl = offerIdSpl.toArrayLike(Buffer, "le", 8);

        const [offerPdaSpl] = PublicKey.findProgramAddressSync(
            [Buffer.from("offer-spl"), userA.publicKey.toBuffer(), idLEspl],
            program.programId
        );

        const vaultSplAta = getAssociatedTokenAddressSync(
            tokenMintA,
            offerPdaSpl,
            true, // allowOwnerOffCurve = true
            TOKEN_PROGRAM_ID,
            ASSOCIATED_TOKEN_PROGRAM_ID
        );

        console.log("Offer SPL PDA =", offerPdaSpl.toBase58());
        console.log("Vault SPL ATA =", vaultSplAta.toBase58());

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

        const tokenAOfferedAmount = new BN(11000000000); // 11 tokens
        const tokenBWantedAmount = new BN(50_000000000); // 50 tokens

        const txSPL = await program.methods.depositSellerSpl(
            offerIdSpl,
            tokenBWantedAmount,
            tokenAOfferedAmount,
        ).accounts({
            maker: userA.publicKey,
            tokenMintA: tokenMintA,
            tokenMintB: tokenMintA,
            makerTokenAccountA: makerTokenAccountA,
            offer: offerPdaSpl,
            vault: vaultSplAta,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        }).signers([userA]).rpc();

        console.log("SPL deposit txn signature:", txSPL);

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
        console.log(
            "Offer stored for SPL deposit, id =",
            offerAccountSpl.id.toString()
        );

    });


});
