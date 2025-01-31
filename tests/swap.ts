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
const userA = Keypair.fromSecretKey(new Uint8Array(require("../chaidex.json")));
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


});
