import { sendAndConfirmTransaction as realSendAndConfirmTransaction } from "@solana/web3.js";
import type {
  Account,
  Connection,
  Transaction,
  TransactionSignature,
} from "@solana/web3.js";

export const sendAndConfirmTransaction = async (
  title: string,
  connection: Connection,
  transaction: Transaction,
  ...signers: Account[]
): Promise<TransactionSignature> => {
  console.debug(`Sending ${title} transaction`);
  let txSig = await realSendAndConfirmTransaction(connection, transaction, signers, {
    skipPreflight: false,
    commitment: "single",
    preflightCommitment: "single",
  });
  console.debug(`TxSig: ${txSig}`)
  return txSig
};
