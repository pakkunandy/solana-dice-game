
import {
  SystemProgram,
  PublicKey,
  Transaction,
  TransactionInstruction,
  LAMPORTS_PER_SOL,
  SYSVAR_RENT_PUBKEY,
} from '@solana/web3.js'

import * as BufferLayout from 'buffer-layout';

import {sendAndConfirmTransaction} from './util/send-and-confirm-transaction';

import {getOurAccount} from './ourAccount'
import {getNodeConnection} from './nodeConnection'
import {getStore} from './storeConfig'


async function main() {

  const ourAccount = await getOurAccount()

  const connection = await getNodeConnection()

  const s = await getStore(connection, 'rejectdups.json')

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
  roll_mode = Buffer.from([roll_mode])

  let threshold = process.argv[3]
  threshold = parseInt(threshold,10)
  threshold = Buffer.from([threshold])

  let bet_value = process.argv[4]
  bet_value = parseInt(bet_value,10)
  const bet_le =  Buffer.allocUnsafe(4)
  bet_le.writeUInt32LE(bet_value,0)
  //console.log(bet_le)

  console.log("-----")

  const instruction_data = Buffer.concat([roll_mode, threshold, bet_le])
  console.log(instruction_data)
  console.log("ProgramId:", s.programId.toString(), "AccountId:", s.accountId.toString())

  if (roll_mode == 1){
    console.log("Roll Under")
  }else {
    console.log("Roll Over");
  }
  console.log("Threshold: ", threshold, " Bet: ", bet_value)
  const balBeforeDice = await connection.getBalance( ourAccount.publicKey )

  //--------------------------------
  // First create check-account...
  //--------------------------------
  console.log("Create Check Acc")
  const seed = 'checkvote'

  const numBytes = 4

  const rentExemption = await connection.getMinimumBalanceForRentExemption(numBytes);
  
  const newAccountPubkey = await PublicKey.createWithSeed(ourAccount.publicKey, seed, s.programId)
  console.log("OK Check Acc")
  
  const alreadyCreated = await connection.getAccountInfo(newAccountPubkey) 

  if ( alreadyCreated ) {
    const data = Buffer.from(alreadyCreated.data)
    const accountDataLayout = BufferLayout.struct([
      BufferLayout.u32('balance'),
    ])
    const bal = accountDataLayout.decode(data)
    console.log("Your balance",bal.balance,"!!!!")
    //process.exit(0)
  } else{
    let params = {

      fromPubkey: ourAccount.publicKey,       // payer
      lamports: rentExemption,                // funds to deposit on the new account
      space: numBytes,                        // space required in bytes
  
      basePubkey: ourAccount.publicKey,       // derive from... must be signer
      seed,                                   // derive from...
      programId: s.programId,                 // derive from... and will be owner of account
  
      newAccountPubkey,
    }
    let createTransaction = new Transaction().add( SystemProgram.createAccountWithSeed( params ) )
  
    await sendAndConfirmTransaction(
      'createAccountWithSeed',
      connection,
      createTransaction,
      ourAccount,            // payer, signer
    )
  }
  
  console.log("Vote check-account created at:",newAccountPubkey.toString(),"for voter:",ourAccount.publicKey.toString())

  //-----------------
  // Then vote.... 
  //-----------------

  // NB: it's possible to be confused about instruction creation, 
  // when we say isSigner: true -- we are making an instruction where that is the case,
  // we are not telling the node if the account signed or not.

  

  const instruction = new TransactionInstruction({
    keys: [
             {pubkey: s.accountId, isSigner: false, isWritable: true},              // contract's data account
             {pubkey: newAccountPubkey, isSigner: false, isWritable: true},         // voter's check-account
             {pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false},      // a system account with rent variables
             {pubkey: ourAccount.publicKey, isSigner: true, isWritable: false}      // voter account 
          ],
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
    BufferLayout.u32('pool')
  ]);

  const prize_pool = accountDataLayout.decode(Buffer.from(accountInfo.data))

  console.log("Prize Pool:", prize_pool.pool) 

  console.log("-----")

  const playerData = Buffer.from(alreadyCreated.data)
  const playerDataLayout = BufferLayout.struct([
    BufferLayout.u32('balance'),
  ])
  const playerBalance = playerDataLayout.decode(playerData)
  console.log("Your balance ",playerBalance.balance,"!!!!")
}

main()
  .catch(err => {
    console.error(err)
  })
  .then(() => process.exit())
