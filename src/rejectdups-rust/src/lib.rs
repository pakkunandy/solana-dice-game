use byteorder::{ByteOrder, LittleEndian};
use num_derive::FromPrimitive;
use solana_program::{
    account_info::next_account_info,
    account_info::AccountInfo,
    decode_error::DecodeError,
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    program_pack::{Pack, Sealed},
    pubkey::Pubkey,
    rent::Rent,
    sysvar::{self, Sysvar},
};
use thiserror::Error;

#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum DiceErr {
    #[error("Unexpected Roll Mode")]
    UnexpectedRollMode,
    #[error("Incrrect Threshold")]
    IncorrectThreshold,
    #[error("Incorrect Owner")]
    IncorrectOwner,
    #[error("Account Not Rent Exempt")]
    AccountNotRentExempt,
    #[error("Account Not Balance Account")]
    AccountNotBalanceAccount,
    #[error("Not Enough Balance")]
    NotEnoughBalance,
    #[error("Invalid Bet")]
    InvalidBet,
}
impl From<DiceErr> for ProgramError {
    fn from(e: DiceErr) -> Self {
        ProgramError::Custom(e as u32)
    }
}
impl<T> DecodeError<T> for DiceErr {
    fn type_of() -> &'static str {
        "Dice Error"
    }
}

// Instruction data

pub struct Dice {
    pub roll_type: u8,
    pub threshold: u8,
    pub bet_amount: u32,
}

impl Sealed for Dice {}

impl Pack for Dice {
    const LEN: usize = 6;

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let roll_type = src[0];
        //println!("Roll Type: {}", roll_type);
        if roll_type != 1 && roll_type != 2 {
            msg!("You should roll under (1) or Roll Over (2)");
            return Err(DiceErr::UnexpectedRollMode.into());
        }

        let threshold = src[1];
        //println!("Threshold: {}", threshold);
        if threshold < 2 || threshold > 98 {
            msg!("Your guess has to in between 2 and 98");
            return Err(DiceErr::IncorrectThreshold.into());
        }

        let bet_amount = LittleEndian::read_u32(&src[2..6]);
        //println!("Bet: {}", bet_amount);
        Ok(Dice { roll_type, threshold, bet_amount})
    }

    fn pack_into_slice(&self, _dst: &mut [u8]) {}
}

// Player's Balance structure, which is one 4 byte u32 number

pub struct PlayerBalance {
    pub balance: u32,
}

impl Sealed for PlayerBalance {}

impl Pack for PlayerBalance {
    const LEN: usize = 4;

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        Ok(PlayerBalance {
            balance: LittleEndian::read_u32(&src[0..4]),
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        LittleEndian::write_u32(&mut dst[0..4], self.balance);
    }
}

// Prize Pool structure, which is a 4 byte u32 number

pub struct PrizePool {
    pub pool_amount: u32,
}

impl Sealed for PrizePool {}

impl Pack for PrizePool {
    const LEN: usize = 4;

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        Ok(PrizePool {
            pool_amount: LittleEndian::read_u32(&src[0..4]),
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        LittleEndian::write_u32(&mut dst[0..4], self.pool_amount);
    }
}

// Declare and export the program's entrypoint
entrypoint!(process_instruction);

// Program entrypoint's implementation
fn process_instruction(
    program_id: &Pubkey,      // Public key of program account
    accounts: &[AccountInfo], // data accounts
    instruction_data: &[u8],  // First Element: Roll type, Second Element: Threshold, [2..6] Bet Amount
) -> ProgramResult {
    msg!("Rust program entrypoint");

    // get Dice information
    let roll_type = Dice::unpack_unchecked(&instruction_data)?.roll_type;
    msg!("Roll Type: {}", roll_type);
    let threshold = Dice::unpack_unchecked(&instruction_data)?.threshold;
    msg!("Threshold: {}", threshold);
    let bet_amount = Dice::unpack_unchecked(&instruction_data)?.bet_amount;
    msg!("Bet: {}", bet_amount);

    // Iterating accounts is safer then indexing
    let accounts_iter = &mut accounts.iter();

    // Get the account that holds the Prize Pool
    let prize_pool_account = next_account_info(accounts_iter)?;
    
    // The account must be owned by the program in order to modify its data
    if prize_pool_account.owner != program_id {
        msg!(
            "Prize Pool account ({}) not owned by program, actual: {}, expected: {}",
            prize_pool_account.key,
            prize_pool_account.owner,
            program_id
        );
        return Err(DiceErr::IncorrectOwner.into());
    }

    // Get the account that holds the balance for the players
    let player_balance_account = next_account_info(accounts_iter)?;

    // The check account must be owned by the program in order to modify its data
    if player_balance_account.owner != program_id {
        msg!("Check account not owned by program");
        return Err(DiceErr::IncorrectOwner.into());
    }
    

    // The account must be rent exempt, i.e. live forever
    let sysvar_account = next_account_info(accounts_iter)?;
    let rent = &Rent::from_account_info(sysvar_account)?;
    if !sysvar::rent::check_id(sysvar_account.key) {
        msg!("Rent system account is not rent system account");
        return Err(ProgramError::InvalidAccountData);
    }
    if !rent.is_exempt(player_balance_account.lamports(), player_balance_account.data_len()) {
        msg!("Balance account is not rent exempt");
        return Err(DiceErr::AccountNotRentExempt.into());
    }
    
    // the player
    let player_account = next_account_info(accounts_iter)?;

    if !player_account.is_signer {
        msg!("Player account is not signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let expected_check_account_pubkey =
        Pubkey::create_with_seed(player_account.key, "checkvote", program_id)?;

    if expected_check_account_pubkey != *player_balance_account.key {
        msg!("Voter fraud! not the correct balance_account");
        return Err(DiceErr::AccountNotBalanceAccount.into());
    }
    
    let mut balance_data = player_balance_account.try_borrow_mut_data()?;

    // this unpack reads and deserialises the account data and also checks the data is the correct length

    let mut player_balance =
        PlayerBalance::unpack_unchecked(&balance_data).expect("Failed to read PlayerBalance");
    

    // Handle the bet_amount and the balance
    /*if vote_check.voted_for != 0 {
        msg!("Voter fraud! You already voted");
        return Err(VoteError::AlreadyVoted.into());
    }*/

    let mut prize_pool_data = prize_pool_account.try_borrow_mut_data()?;
    
    let mut prize_pool =
        PrizePool::unpack_unchecked(&prize_pool_data).expect("Failed to read PrizePool");
    
    ///////////////////////
    // Jut for Debug
    if player_balance.balance == 0 {
        msg!{"Airdrop some money!!!"};
        player_balance.balance = 50; 
    }
    if prize_pool.pool_amount == 0 {
        msg!{"Airdrop some money!!!"};
        prize_pool.pool_amount = 1000; 
    }

    // Check the valid of the bet amount
    if bet_amount > player_balance.balance {
        msg!("Not Enough Balance");
        return Err(DiceErr::NotEnoughBalance.into());
    }
    if bet_amount == 0 {
        msg!("Inavalid Bet");
        return Err(DiceErr::InvalidBet.into());
    }

    let lucky_number:u8 = 20;
    println!("Result {}", lucky_number);
    let mut win_amount:u32 = 0;

    if (1 == roll_type && lucky_number <= threshold) || (2 == roll_type && lucky_number >= threshold) {
        if lucky_number <= 25 || lucky_number >= 75 {
            win_amount = bet_amount as u32 * 2;  
            msg!("Win: {}", win_amount);          
        }else{
            win_amount = bet_amount as u32;
            msg!("Win: {}", win_amount);
        }
    }
    if win_amount == 0 {
        prize_pool.pool_amount += bet_amount;
        player_balance.balance -= bet_amount;
        msg!("You Lose!");
    }else{
        prize_pool.pool_amount -= win_amount;
        player_balance.balance += win_amount;
        msg!("You Win!");
    }

    PrizePool::pack(prize_pool, &mut prize_pool_data).expect("Failed to write Prize Pool");
    PlayerBalance::pack(player_balance, &mut balance_data).expect("Failed to write Player Balance");

    Ok(())
}


#[cfg(test)]
mod test {
    use super::*;
    
    use solana_program::instruction::InstructionError::Custom;
    use solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        sysvar,
    };
    use solana_program_test::*;
    use solana_sdk::transaction::TransactionError;
    use solana_sdk::{
        account::Account,
        signature::{Keypair, Signer},
        transaction::Transaction,
    };
    use std::mem;

    use self::tokio;


    impl From<DiceErr> for TransactionError {
        fn from(e: DiceErr) -> Self {
            TransactionError::InstructionError(0, Custom(e as u32))
        }
    }
    

    #[tokio::test]
    async fn test_sanity1() {
        //++++++++++++++++++++++++++++++++++++
        // TEST: Simply vote for Bet
        //++++++++++++++++++++++++++++++++++++

        let program_id = Pubkey::new_unique();

        let mut program_test =
            ProgramTest::new("dice", program_id, processor!(process_instruction));

        // mock contract data account
        let game_key = Pubkey::new_unique();
        let mut data: Vec<u8> = vec![0; 4 * mem::size_of::<u8>()];
        LittleEndian::write_u32(&mut data[0..4], 1000); // set prize pool to 1000
        println!("Prize Pool {:?}", data);
        program_test.add_account(
            game_key,
            Account {
                lamports: 60000,
                data,
                owner: program_id,
                executable: false,
                rent_epoch: 0,
            },
        );

        // player account
        let player_keypair = Keypair::new();
        let player_key = player_keypair.pubkey();

        // mock player balance_account_data
        let balance_key = Pubkey::create_with_seed(&player_key, "checkvote", &program_id).unwrap(); // derived (correctly)
        let mut data = vec![0; mem::size_of::<u32>()];
        LittleEndian::write_u32(&mut data[0..4], 50); // set storage to 50
        program_test.add_account(
            balance_key,
            Account {
                lamports: 1000000,
                data,
                owner: program_id,
                executable: false,
                rent_epoch: 0,
            },
        );

        let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

        let game_account = banks_client.get_account(game_key).await.unwrap().unwrap();
        let prize_bool_amount =
            PrizePool::unpack_unchecked(&game_account.data).expect("Failed to read Prize Pool");
        assert_eq!(prize_bool_amount.pool_amount, 1000);

        // Roll Under
        let accounts = vec![
            AccountMeta::new(game_key, false),
            AccountMeta::new(balance_key, false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new(player_key, true),
        ];

        let mut bet = vec![0; 6*mem::size_of::<u8>()];
        bet[0] = 1; // Role Under
        bet[1] = 30; // Threshold 30
        LittleEndian::write_u32(&mut bet[2..6], 10); // Bet 10
        println!("Instruction Data {:?}", bet);
        let mut transaction = Transaction::new_with_payer(
            &[Instruction {
                program_id,
                accounts,
                data: bet,
            }],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&payer, &player_keypair], recent_blockhash);

        let a = banks_client.process_transaction(transaction).await;

        println!("Test Log {:?}", a);
        let game_account = banks_client.get_account(game_key).await.unwrap().unwrap();
        let prize_pool_check =
            PrizePool::unpack_unchecked(&game_account.data).expect("Failed to read Prize Pool");
        assert_eq!(prize_pool_check.pool_amount, 980);

        let player = banks_client.get_account(balance_key).await.unwrap().unwrap();
        let bal_check =
        PlayerBalance::unpack_unchecked(&player.data).expect("Failed to read Balance");
        assert_eq!(bal_check.balance, 70);
    }
}
