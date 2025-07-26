use borsh::BorshSerialize;
use solana_program::{
    pubkey::Pubkey,
    account_info::{AccountInfo, next_account_info},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    sysvar::{Sysvar, rent::Rent},
    program::{invoke, invoke_signed},
    borsh1::try_from_slice_unchecked,
    program_pack::Pack,
};
use solana_system_interface::instruction::{
    create_account,
    transfer,
};
use spl_associated_token_account::{
    get_associated_token_address,
    instruction::create_associated_token_account_idempotent,
};
use spl_token::{
    instruction::transfer_checked,
    state::Mint,
};

use crate::{
    instruction::SwapInstruction,
    state::LiquidityPool,
    error::SwapProgramError,
};


pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8]
) -> ProgramResult {
    let instruction = SwapInstruction::unpack(instruction_data)?;

    match instruction {
        SwapInstruction::CreatePool => {
            process_create_pool(program_id, accounts)
        },
        SwapInstruction::FundPool { amount } => {
            process_fund_pool(program_id, accounts, amount)
        },
        SwapInstruction::Swap { amount_to_swap } => {
            process_swap(program_id, accounts, amount_to_swap)
        }
    }
}

pub fn process_create_pool(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let pool = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    let (pool_pda, pool_bump) = Pubkey::find_program_address(
        &[LiquidityPool::SEED_PREFIX.as_bytes()], program_id);

    if *pool.key != pool_pda {
        return Err(ProgramError::InvalidSeeds);
    }

    let rent = Rent::get()?;

    let pool_rent = rent.minimum_balance(LiquidityPool::SPACE);

    invoke_signed(
        &create_account(
            payer.key, 
            pool.key, 
            pool_rent, 
            LiquidityPool::SPACE as u64, 
            program_id,
        ), 
        &[payer.clone(), pool.clone(), system_program.clone()],
        &[
            &[LiquidityPool::SEED_PREFIX.as_bytes(), &[pool_bump]]
        ]
    )?;

    let mut pool_data = 
        try_from_slice_unchecked::<LiquidityPool>(&pool.data.borrow())?;

    pool_data.assets = vec![];
    pool_data.bump = pool_bump;

    pool_data.serialize(&mut &mut pool.data.borrow_mut()[..])?;

    Ok(())
}

pub fn process_fund_pool(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let pool = next_account_info(accounts_iter)?;
    let mint = next_account_info(accounts_iter)?;
    let pool_ata = next_account_info(accounts_iter)?;
    let payer_ata = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;
    let associated_token_program = next_account_info(accounts_iter)?;

    let (pool_pda, pool_bump) = Pubkey::find_program_address
        (&[LiquidityPool::SEED_PREFIX.as_bytes()], program_id);

    if *pool.key != pool_pda {
        return Err(ProgramError::InvalidSeeds);
    }

    if *pool_ata.key != get_associated_token_address(pool.key, mint.key) {
        return Err(ProgramError::InvalidSeeds);
    }

    if *payer_ata.key != get_associated_token_address(payer.key, mint.key) {
        return Err(ProgramError::InvalidSeeds);
    }

    invoke(
        &create_associated_token_account_idempotent(
            payer.key, 
            pool.key, 
            mint.key, 
            token_program.key,
        ), 
        &[payer.clone(), pool.clone(), mint.clone(), token_program.clone()], 
    )?;

    let mut pool_data = try_from_slice_unchecked::<LiquidityPool>(&pool.data.borrow())?;

    if !pool_data.assets.contains(mint.key) {
        let rent = Rent::get()?;

        let new_account_size = pool.data_len() + 32;

        let lamports_required = rent.minimum_balance(new_account_size);
        let additional_rent_to_fund = lamports_required - pool.lamports();

        invoke(
            &transfer(
                payer.key, 
                pool.key, 
                additional_rent_to_fund,
            ), 
            &[payer.clone(), pool.clone()],
        )?;

        pool.resize(new_account_size)?;

        pool_data.assets.push(*mint.key);
    } 

    let mint_data = Mint::unpack(&mint.data.borrow())?;

    invoke(
        &transfer_checked(
            token_program.key, 
            payer_ata.key, 
            mint.key, 
            pool_ata.key, 
            payer.key, 
            &[], 
            amount, 
            mint_data.decimals
        )?, 
        &[token_program.clone(), payer_ata.clone(), mint.clone(), pool_ata.clone(), payer.clone()],
    )?;

    Ok(())
}

pub fn process_swap(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount_to_swap: u64
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let pool = next_account_info(accounts_iter)?;
    let receive_mint = next_account_info(accounts_iter)?;
    let pool_receive_ata = next_account_info(accounts_iter)?;
    let payer_receive_ata = next_account_info(accounts_iter)?;
    let pay_mint = next_account_info(accounts_iter)?;
    let pool_pay_ata = next_account_info(accounts_iter)?;
    let payer_pay_ata = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;
    let associated_token_program = next_account_info(accounts_iter)?;

    let (pool_pda, pool_bump) = Pubkey::find_program_address
        (&[LiquidityPool::SEED_PREFIX.as_bytes()], program_id);

    if *pool_receive_ata.key != get_associated_token_address(pool.key, receive_mint.key) {
        return Err(ProgramError::InvalidSeeds);
    }

    if *payer_receive_ata.key != get_associated_token_address(payer.key, receive_mint.key) {
        return Err(ProgramError::InvalidSeeds);
    }

    if *pool_pay_ata.key != get_associated_token_address(pool.key, pay_mint.key) {
        return Err(ProgramError::InvalidSeeds);
    }

    if *payer_pay_ata.key != get_associated_token_address(payer.key, pay_mint.key) {
        return Err(ProgramError::InvalidSeeds);
    }

    if amount_to_swap == 0 {
        return Err(SwapProgramError::InvalidSwapZeroAmount.into());
    }

    if *receive_mint.key == *pay_mint.key {
        return Err(SwapProgramError::InvalidSwapMatchingAssets.into());
    }


    Ok(())
}