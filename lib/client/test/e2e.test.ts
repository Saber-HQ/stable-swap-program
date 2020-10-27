import fs from "fs";

import { Token } from "@solana/spl-token";
import { Account, Connection, PublicKey } from "@solana/web3.js";

import { StableSwap } from "../src";
import { NumberU64 } from "../src/util/u64"
import { newAccountWithLamports, sleep } from "./helpers";

// Token Program
const TokenProgramId: PublicKey = new PublicKey(
  "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
);
// Cluster configs
const CLUSTER_URL = "http://localhost:8899";
const BOOTSTRAP_TIMEOUT = 300000;
// Pool configs
const AMP_FACTOR = new NumberU64(100);
const FEE_NUMERATOR = new NumberU64(1);
const FEE_DENOMINATOR = new NumberU64(4);
//  Other constants
const oneSol = 1000000000;

const getStableSwapAddress = (): string => {
  let data = fs.readFileSync("../../localnet-address.json", "utf-8");
  let addresses = JSON.parse(data);
  return addresses.stableSwap as string;
};

describe("e2e test", () => {
  // Cluster connection
  let connection: Connection;
  // Fee payer
  let payer: Account;
  // authority of the token and accounts
  let authority: PublicKey;
  // nonce used to generate the authority public key
  let nonce: number;
  // owner of the user accounts
  let owner: Account;
  // Token pool
  let tokenPool: Token;
  let tokenAccountPool: PublicKey;
  // Tokens swapped
  let mintA: Token;
  let mintB: Token;
  let tokenAccountA: PublicKey;
  let tokenAccountB: PublicKey;
  // Stable swap
  let stableSwap: StableSwap;
  let stableSwapAccount: Account;
  let stableSwapProgramId: PublicKey;

  beforeAll(async (done) => {
    // Bootstrap Test Environment ...
    connection = new Connection(CLUSTER_URL, "single");

    stableSwapProgramId = new PublicKey(getStableSwapAddress());

    console.log("Token Program ID", TokenProgramId.toString());
    console.log("StableSwap Program ID", stableSwapProgramId.toString());
    payer = await newAccountWithLamports(connection, oneSol);
    owner = await newAccountWithLamports(connection, oneSol);
    stableSwapAccount = new Account();
    try {
      [authority, nonce] = await PublicKey.findProgramAddress(
        [stableSwapAccount.publicKey.toBuffer()],
        stableSwapProgramId
      );
    } catch (e) {
      console.error(e);
    }

    console.log("creating pool mint");
    try {
      tokenPool = await Token.createMint(
        connection,
        payer,
        authority,
        null,
        2,
        TokenProgramId
      );
    } catch (e) {
      console.error(e);
    }

    console.log('creating pool account');
    try {
      tokenAccountPool = await tokenPool.createAccount(owner.publicKey);
    } catch(e) {
      console.error(e);
    }

    console.log("creating token A");
    try {
      mintA = await Token.createMint(
        connection,
        payer,
        owner.publicKey,
        null,
        2,
        TokenProgramId
      );
    } catch (e) {
      console.error(e);
    }

    console.log("creating token A account");
    try {
      tokenAccountA = await mintA.createAccount(authority);
    } catch (e) {
      console.error(e);
    }

    console.log("creating token B");
    try {
      mintB = await Token.createMint(
        connection,
        payer,
        owner.publicKey,
        null,
        2,
        TokenProgramId
      );
    } catch (e) {
      console.error(e);
    }

    console.log("creating token B account");
    try {
      tokenAccountB = await mintB.createAccount(authority);
    } catch (e) {
      console.error(e);
    }

    // Sleep to make sure token accounts are created ...
    await sleep(500);

    console.log("creating token swap");
    try {
      stableSwap = await StableSwap.createStableSwap(
        connection,
        payer,
        stableSwapAccount,
        authority,
        tokenAccountA,
        tokenAccountB,
        tokenPool.publicKey,
        mintA.publicKey,
        mintB.publicKey,
        stableSwapProgramId,
        TokenProgramId,
        nonce,
        AMP_FACTOR,
        FEE_NUMERATOR,
        FEE_DENOMINATOR
      );
    } catch (e) {
      console.error(e);
    }

    done();
  }, BOOTSTRAP_TIMEOUT);

  it("loadStableSwap", async () => {
    const fetchedStableSwap = await StableSwap.loadStableSwap(
      connection,
      stableSwapAccount.publicKey,
      stableSwapProgramId,
      payer
    );
    expect(fetchedStableSwap.stableSwap).toEqual(stableSwapAccount.publicKey);
    expect(fetchedStableSwap.tokenAccountA).toEqual(tokenAccountA);
    expect(fetchedStableSwap.tokenAccountB).toEqual(tokenAccountB);
    // TODO: Put mints back to swap state
    // expect(fetchedStableSwap.mintA).toEqual(mintA.publicKey);
    // expect(fetchedStableSwap.mintB).toEqual(mintB.publicKey);
    expect(fetchedStableSwap.poolToken).toEqual(tokenPool.publicKey);
    expect(fetchedStableSwap.ampFactor).toEqual(AMP_FACTOR);
    expect(fetchedStableSwap.feeNumerator).toEqual(FEE_NUMERATOR);
    expect(fetchedStableSwap.feeDenominator).toEqual(FEE_DENOMINATOR);
  });

  it("deposit", async () => {
    const depositAmountA = oneSol;
    const depositAmountB = oneSol; 
    console.log('Creating depositor token a account');
    const userAccountA = await mintA.createAccount(owner.publicKey);
    await mintA.mintTo(userAccountA, owner, [], depositAmountA);
    await mintA.approve(userAccountA, authority, owner, [], depositAmountA);
    console.log('Creating depositor token b account');
    const userAccountB = await mintB.createAccount(owner.publicKey);
    await mintB.mintTo(userAccountB, owner, [], depositAmountB);
    await mintB.approve(userAccountB, authority, owner, [], depositAmountB);
    console.log('Creating depositor pool token account');
    const newAccountPool = await tokenPool.createAccount(owner.publicKey);

    console.log('Depositing into swap');
    try {
      await stableSwap.deposit(
        userAccountA,
        userAccountB,
        newAccountPool,
        depositAmountA,
        depositAmountB,
        0,   // To avoid slippage errors
      );
    } catch(e) {
      throw new Error(e)
    }

    let info = await mintA.getAccountInfo(userAccountA);
    expect(info.amount.toNumber()).toBe(0);
    info = await mintB.getAccountInfo(userAccountB);
    expect(info.amount.toNumber()).toBe(0);
    info = await mintA.getAccountInfo(tokenAccountA);
    expect(info.amount.toNumber()).toBe(depositAmountA);
    info = await mintB.getAccountInfo(tokenAccountB);
    expect(info.amount.toNumber()).toBe(depositAmountB);
    info = await tokenPool.getAccountInfo(newAccountPool);
    expect(info.amount.toNumber()).toBe(1);
  })
});
