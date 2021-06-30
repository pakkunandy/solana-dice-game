use byteorder::{ByteOrder, LittleEndian};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use std::mem;
// use rand::Rng;

// Declare and export the program's entrypoint
entrypoint!(process_instruction);

// Program entrypoint's implementation
fn process_instruction(
    program_id: &Pubkey,      // Public key of program account
    accounts: &[AccountInfo], // balance account
    instruction_data: &[u8],  // First Element: Roll type, Second Element: Threshold, Third Element: Bet Amount
) -> ProgramResult {
    msg!("Rust program entrypoint");

    // Iterating accounts is safer then indexing
    let accounts_iter = &mut accounts.iter();

    // Get the account that holds the vote count
    let account = next_account_info(accounts_iter)?;

    // The account must be owned by the program in order to modify its data
    if account.owner != program_id {
        msg!("Vote account is not owned by the program");
        return Err(ProgramError::IncorrectProgramId);
    }

    let mut data = account.try_borrow_mut_data()?;

    //Random number
    //let mut rng = rand::thread_rng();
    //let mut result: u8 = rng.gen_range(0..101);
    
    let result:u8 = 20;
    println!("Result {}", result);

    // Get the guessed thresold
    let threshold = instruction_data[1];
    let bet_value = instruction_data[2];

    let mut win_amount:u32;

    if (1 == instruction_data[0] && result <= threshold) || (2 == instruction_data[0] && result >= threshold) {
        if result <= 25 || result >= 75 {
            win_amount = bet_value as u32 * 3;  
            println!("Win: {}", win_amount);          
        }else{
            win_amount = bet_value as u32;
            println!("Win: {}", win_amount);
        }
        let balance = LittleEndian::read_u32(&data[0..4]);
        LittleEndian::write_u32(&mut data[0..4], balance + win_amount);
        msg!("Win!!!");
    }
    Ok(())
}

// tests

#[cfg(test)]
mod test {
    use super::*;
    use solana_program::clock::Epoch;

    #[test]
    fn test_sanity() {
        // mock program id

        let program_id = Pubkey::default();

        // mock accounts array...

        let key = Pubkey::default(); // anything
        let mut lamports = 0;

        let mut data = vec![0; mem::size_of::<u32>()];
        LittleEndian::write_u32(&mut data[0..4], 0); // set balance to zero

        let owner = Pubkey::default();

        let account = AccountInfo::new(
            &key,             // account pubkey
            false,            // is_signer
            true,             // is_writable
            &mut lamports,    // balance in lamports
            &mut data,        // storage
            &owner,           // owner pubkey
            false,            // is_executable
            Epoch::default(), // rent_epoch
        );

        
        let accounts = vec![account];

        assert_eq!(LittleEndian::read_u32(&accounts[0].data.borrow()[0..4]), 0);

        // Roll Under
        let mut instruction_data: Vec<u8> = vec![1, 50, 10];
        process_instruction(&program_id, &accounts, &instruction_data).unwrap();
        assert_eq!(LittleEndian::read_u32(&accounts[0].data.borrow()[0..4]), 30);

        // Roll Over

        instruction_data = vec![2, 30, 25];
        process_instruction(&program_id, &accounts, &instruction_data).unwrap();
        assert_eq!(LittleEndian::read_u32(&accounts[0].data.borrow()[0..4]), 30);
    }
}
