
import {
  Transaction,
  TransactionInstruction,
  LAMPORTS_PER_SOL,
} from '@solana/web3.js'

import * as BufferLayout from 'buffer-layout';

import {sendAndConfirmTransaction} from './util/send-and-confirm-transaction';

import {getOurAccount} from './ourAccount'
import {getNodeConnection} from './nodeConnection'
import {getStore} from './storeConfig'


async function main() {

  const ourAccount = await getOurAccount()

  const connection = await getNodeConnection()

  const s = await getStore(connection, 'simplest.json')

  if ( !s.inStore ) {
    console.log("Deploy program first")
    process.exit(1)
  }

  let roll_mode = process.argv[2];

  if ( ! roll_mode || roll_mode !== "1" && roll_mode !== "2" ) {
    console.log("No mode supported (should be 1 or 2)");
    process.exit(1);
  }

  roll_mode = parseInt(roll_mode,10)

  let threshold = process.argv[3]
  let bet_value = process.argv[4]
  threshold = parseInt(threshold,10)
  bet_value = parseInt(bet_value,10)

  console.log("-----")
  console.log(roll_mode)
  const instruction_data = Buffer.from([roll_mode, threshold, bet_value])

  console.log("ProgramId:", s.programId.toString(), "AccountId:", s.accountId.toString())

  if (roll_mode == 1){
    console.log("Roll Under")
  }else {
    console.log("Roll Over");
  }
  console.log("Threshold: ", threshold, " Bet: ", bet_value)
  const balBeforeDice = await connection.getBalance( ourAccount.publicKey )

  const instruction = new TransactionInstruction({
    keys: [{pubkey: s.accountId, isSigner: false, isWritable: true}],
    programId: s.programId,
    data: instruction_data,
  })
  await sendAndConfirmTransaction(
    'dice',
    connection,
    new Transaction().add(instruction),
    ourAccount,
  )

  const balAfterDice = await connection.getBalance( ourAccount.publicKey )

  const costOfDice = balAfterDice - balAfterDice

  console.log("Cost of dicing:",costOfDice,"lamports (", costOfDice/LAMPORTS_PER_SOL, ")")

  const accountInfo = await connection.getAccountInfo(s.accountId)
  const data = Buffer.from(accountInfo.data)
  
  const accountDataLayout = BufferLayout.struct([
    BufferLayout.u32('money')
  ]);

  const counts = accountDataLayout.decode(Buffer.from(accountInfo.data))

  console.log("Current money:", counts.money) 

  console.log("-----")
}

main()
  .catch(err => {
    console.error(err)
  })
  .then(() => process.exit())
