use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo}, 
    entrypoint::ProgramResult, 
    program_error::ProgramError,
    program::{invoke, invoke_signed}, 
    program_pack::Pack, 
    pubkey::Pubkey, 
    sysvar::{rent::Rent, Sysvar},
    borsh1::try_from_slice_unchecked,
    msg,
};

use solana_system_interface::{
    program::id as system_program_id,
    instruction::create_account,
};

use spl_associated_token_account::{
    id as associated_token_program_id,
    get_associated_token_address,
    instruction::{create_associated_token_account, create_associated_token_account_idempotent},
};
use spl_token::{
    id as token_program_id,
    instruction::{transfer_checked, initialize_mint2, mint_to, burn},
    state::Mint,
};

use integer_sqrt::IntegerSquareRoot;

use crate::{
    instruction::AmmInstruction,
    state::LiquidityPool,
    error::AmmError,
};

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8]
) -> ProgramResult {
    let instruction = AmmInstruction::unpack(instruction_data)?;

    match instruction {
        AmmInstruction::CreatePool { amount_a, amount_b, fee_bps } => {
            process_create_pool(program_id, accounts, amount_a, amount_b, fee_bps)
        },
        AmmInstruction::ProvideLiquidity { amount_a_desired, amount_b_desired, amount_a_min, amount_b_min } => {
            process_provide_liquidity(program_id, accounts, amount_a_desired, amount_b_desired, amount_a_min, amount_b_min)
        },
        AmmInstruction::WithdrawLiquidity { amount_lp_in, amount_a_min, amount_b_min } => {
            process_withdraw_liquidity(program_id, accounts, amount_lp_in, amount_a_min, amount_b_min)
        },
        AmmInstruction::Swap { amount_in, min_out } => {
            process_swap(program_id, accounts, amount_in, min_out)
        },
    }
}

pub fn process_create_pool(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount_a: u64,
    amount_b: u64,
    fee_bps: u16,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let user = next_account_info(accounts_iter)?;
    let pool = next_account_info(accounts_iter)?;
    let mint_a = next_account_info(accounts_iter)?;
    let mint_b = next_account_info(accounts_iter)?;
    let vault_a = next_account_info(accounts_iter)?;
    let vault_b = next_account_info(accounts_iter)?;
    let mint_lp = next_account_info(accounts_iter)?;
    let user_ata_lp = next_account_info(accounts_iter)?;
    let user_ata_a = next_account_info(accounts_iter)?;
    let user_ata_b = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;
    let associated_token_program = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    if !user.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if mint_a.key == mint_b.key {
        return Err(AmmError::IdenticalMints.into());
    }

    let (mint_lo, mint_hi) = if mint_a.key < mint_b.key {
        (mint_a.key.clone(), mint_b.key.clone())
    } else {
        (mint_b.key.clone(), mint_a.key.clone())
    };

    let (pool_pda, pool_bump) = Pubkey::find_program_address(
        &[b"pool", mint_lo.as_ref(), mint_hi.as_ref(), &fee_bps.to_le_bytes()], 
        program_id,
    );

    if *pool.key != pool_pda {
        return Err(AmmError::PoolAddressMismatch.into());
    }

    if *vault_a.key != get_associated_token_address(pool.key, mint_a.key) {
        return Err(AmmError::VaultAddressMismatch.into());
    }

    if *vault_b.key != get_associated_token_address(pool.key, mint_b.key) {
        return Err(AmmError::VaultAddressMismatch.into());
    }

    let (lp_mint_pda, lp_mint_bump) = Pubkey::find_program_address(
        &[b"lp_mint", pool.key.as_ref()], program_id);

    if *mint_lp.key != lp_mint_pda {
        return Err(AmmError::LpMintAddressMismatch.into());
    }

    if *token_program.key != token_program_id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    if *associated_token_program.key != associated_token_program_id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    if *system_program.key != system_program_id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    if amount_a == 0 || amount_b == 0 {
        return Err(AmmError::ZeroLiquidityAmount.into());
    }

    if fee_bps > 10_000 {
        return Err(AmmError::FeeTooHigh.into());
    }

    // create pool account
    let rent = Rent::get()?;

    let pool_rent = rent.minimum_balance(LiquidityPool::SPACE);

    invoke_signed(
        &create_account(
            user.key, 
            pool.key, 
            pool_rent, 
            LiquidityPool::SPACE as u64, 
            program_id,
        ), 
        &[user.clone(), pool.clone()], 
        &[
            &[
                b"pool",
                mint_lo.as_ref(),
                mint_hi.as_ref(),
                &fee_bps.to_le_bytes(),
                &[pool_bump],
            ]
        ],
    )?;

    // create vault_a ( pool's ata for mint_a )
    invoke(
        &create_associated_token_account(
            user.key, 
            pool.key, 
            mint_a.key, 
            token_program.key,
        ), 
        &[user.clone(), vault_a.clone(), pool.clone(), mint_a.clone()],
    )?;

    // create vault_v ( pool's ata for mint_b )
    invoke(
        &create_associated_token_account(
            user.key, 
            pool.key, 
            mint_b.key, 
            token_program.key,
        ), 
        &[user.clone(), vault_b.clone(), pool.clone(), mint_b.clone()],
    )?;

    // transfer amount_a from user_ata_a to vault_a
    let mint_a_data = Mint::unpack(&mint_a.data.borrow())?;

    invoke(
        &transfer_checked(
            token_program.key, 
            user_ata_a.key, 
            mint_a.key, 
            vault_a.key, 
            user.key, 
            &[], 
            amount_a, 
            mint_a_data.decimals,
        )?, 
        &[user_ata_a.clone(), mint_a.clone(), vault_a.clone(), user.clone()], 
    )?;
    
    // transfer amount_b from user ata to pool ata
    let mint_b_data = Mint::unpack(&mint_b.data.borrow())?;

    invoke(
        &transfer_checked(
            token_program.key, 
            user_ata_b.key, 
            mint_b.key, 
            vault_b.key, 
            user.key, 
            &[], 
            amount_b, 
            mint_b_data.decimals,
        )?, 
        &[user_ata_b.clone(), mint_b.clone(), vault_b.clone(), user.clone()], 
    )?;

    // create mint_lp
    let mint_rent = rent.minimum_balance(Mint::LEN);

    invoke_signed(
        &create_account(
            user.key, 
            mint_lp.key, 
            mint_rent, 
            Mint::LEN as u64, 
            token_program.key,
        ), 
        &[user.clone(), mint_lp.clone()], 
        &[
            &[b"lp_mint", pool.key.as_ref(), &[lp_mint_bump]],
        ],
    )?;

    invoke(
        &initialize_mint2(
            token_program.key, 
            mint_lp.key, 
            pool.key, 
            None, 
            9,
        )?, 
        &[mint_lp.clone(), pool.clone()],
    )?;

    // create user_ata_lp
    invoke(
        &create_associated_token_account_idempotent(
            user.key, 
            user.key, 
            mint_lp.key, 
            token_program.key,
        ), 
        &[user.clone(), user_ata_lp.clone(), mint_lp.clone()],
    )?;

    // mint lp tokens to user_ata_lp
    let lp_amount = (amount_a as u128)
        .checked_mul(amount_b as u128)
        .ok_or(ProgramError::InvalidInstructionData)?
        .integer_sqrt() as u64;

    invoke_signed(
        &mint_to(
            token_program.key, 
            mint_lp.key, 
            user_ata_lp.key, 
            pool.key, 
            &[], 
            lp_amount,
        )?, 
        &[mint_lp.clone(), user_ata_lp.clone(), pool.clone()], 
        &[
            &[b"pool", mint_lo.as_ref(), mint_hi.as_ref(), &fee_bps.to_le_bytes(), &[pool_bump]],
        ]
    )?;

    // update pool data
    let mut pool_data = 
        try_from_slice_unchecked::<LiquidityPool>(&pool.data.borrow())?;

    pool_data.mint_a = *mint_a.key;
    pool_data.mint_b = *mint_b.key;
    pool_data.reserve_a = amount_a;
    pool_data.reserve_b = amount_b;
    pool_data.fee_bps = fee_bps;
    pool_data.bump = pool_bump;

    pool_data.serialize(&mut &mut pool.data.borrow_mut()[..])?;

    Ok(())
}

pub fn process_provide_liquidity(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount_a_desired: u64,
    amount_b_desired: u64,
    amount_a_min: u64,
    amount_b_min: u64,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let user = next_account_info(accounts_iter)?;
    let pool = next_account_info(accounts_iter)?;
    let mint_a = next_account_info(accounts_iter)?;
    let mint_b = next_account_info(accounts_iter)?;
    let vault_a = next_account_info(accounts_iter)?;
    let vault_b = next_account_info(accounts_iter)?;
    let mint_lp = next_account_info(accounts_iter)?;
    let user_ata_lp = next_account_info(accounts_iter)?;
    let user_ata_a = next_account_info(accounts_iter)?;
    let user_ata_b = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;

    if !user.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut pool_data = 
        try_from_slice_unchecked::<LiquidityPool>(&pool.data.borrow())?;
    
    let (mint_lo, mint_hi) = if pool_data.mint_a < pool_data.mint_b {
        (pool_data.mint_a, pool_data.mint_b)
    } else {
        (pool_data.mint_b, pool_data.mint_a)
    };

    let expected_pool = Pubkey::create_program_address(
        &[b"pool", mint_lo.as_ref(), mint_hi.as_ref(), &pool_data.fee_bps.to_le_bytes(), &[pool_data.bump]], 
        program_id,
    ).map_err(|_| ProgramError::InvalidSeeds)?;

    if expected_pool != *pool.key {
        return Err(AmmError::PoolAddressMismatch.into());
    }

    if *mint_a.key != pool_data.mint_a {
        return Err(AmmError::MintAddressMismatch.into());
    }

    if *mint_b.key != pool_data.mint_b {
        return Err(AmmError::MintAddressMismatch.into());
    }

    if *vault_a.key != get_associated_token_address(pool.key, mint_a.key) {
        return Err(AmmError::VaultAddressMismatch.into());
    }

    if *vault_b.key != get_associated_token_address(pool.key, mint_b.key) {
        return Err(AmmError::VaultAddressMismatch.into());
    }

    let (expected_lp_mint, _lp_mint_bump) = Pubkey::find_program_address(
        &[b"lp_mint", pool.key.as_ref()], program_id);

    if *mint_lp.key != expected_lp_mint {
        return Err(AmmError::LpMintAddressMismatch.into());
    }

    if amount_a_desired == 0 || amount_b_desired == 0 {
        return Err(AmmError::ZeroLiquidityAmount.into());
    }

    let reserve_a = pool_data.reserve_a as u128;
    let reserve_b = pool_data.reserve_b as u128;
    let amount_a_desired = amount_a_desired as u128;
    let amount_b_desired = amount_b_desired as u128;

    let take_a;
    let take_b;
    
    let b_needed = 
        amount_a_desired.checked_mul(reserve_b).ok_or(ProgramError::ArithmeticOverflow)?
        / reserve_a;

    if b_needed <= amount_b_desired {
        take_a = amount_a_desired;
        take_b = b_needed
    } else {
        take_b = amount_b_desired;
        take_a = 
            amount_b_desired.checked_mul(reserve_a).ok_or(ProgramError::ArithmeticOverflow)?
            / reserve_b;
    }

    if take_a < amount_a_min as u128 || take_b < amount_b_min as u128{
        return Err(AmmError::SlippageExceed.into());
    }

    // calculate lp tokens to mint
    let total_lp = Mint::unpack(&mint_lp.data.borrow())?.supply as u128;

    let lp_from_a = take_a * total_lp / reserve_a;
    let lp_from_b = take_b * total_lp / reserve_b;
    let lp_amount = core::cmp::min(lp_from_a, lp_from_b) as u64;

    let take_a = u64::try_from(take_a).map_err(|_| ProgramError::ArithmeticOverflow)?;
    let take_b = u64::try_from(take_b).map_err(|_| ProgramError::ArithmeticOverflow)?;

    let mint_a_data = Mint::unpack(&mint_a.data.borrow())?;
    let mint_b_data = Mint::unpack(&mint_b.data.borrow())?;

    // transfer take_a amount from user_ata_a to vault_a
    invoke(
        &transfer_checked(
            token_program.key, 
            user_ata_a.key, 
            mint_a.key, 
            vault_a.key, 
            user.key, 
            &[], 
            take_a, 
            mint_a_data.decimals,
        )?, 
        &[user_ata_a.clone(), mint_a.clone(), vault_a.clone(), user.clone()],
    )?;

    // transfer take_b amount from user_ata_b to vault_b
    invoke(
        &transfer_checked(
            token_program.key, 
            user_ata_b.key, 
            mint_b.key, 
            vault_b.key, 
            user.key, 
            &[], 
            take_b, 
            mint_b_data.decimals,
        )?, 
        &[user_ata_b.clone(), mint_b.clone(), vault_b.clone(), user.clone()],
    )?;

    // mint lp tokens to user
    invoke_signed(
        &mint_to(
            token_program.key, 
            mint_lp.key, 
            user_ata_lp.key, 
            pool.key, 
            &[], 
            lp_amount,
        )?, 
        &[mint_lp.clone(), user_ata_lp.clone(), pool.clone()], 
        &[
            &[b"pool", mint_lo.as_ref(), mint_hi.as_ref(), &pool_data.fee_bps.to_le_bytes(), &[pool_data.bump]],
        ]
    )?;

    pool_data.reserve_a = pool_data.reserve_a.checked_add(take_a).ok_or(ProgramError::ArithmeticOverflow)?;
    pool_data.reserve_b = pool_data.reserve_b.checked_add(take_b).ok_or(ProgramError::ArithmeticOverflow)?;

    pool_data.serialize(&mut &mut pool.data.borrow_mut()[..])?;

    Ok(())
}

pub fn process_withdraw_liquidity(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount_lp_in: u64,
    amount_a_min: u64,
    amount_b_min: u64,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let user = next_account_info(accounts_iter)?;
    let pool = next_account_info(accounts_iter)?;
    let mint_a = next_account_info(accounts_iter)?;
    let mint_b = next_account_info(accounts_iter)?;
    let vault_a = next_account_info(accounts_iter)?;
    let vault_b = next_account_info(accounts_iter)?;
    let mint_lp = next_account_info(accounts_iter)?;
    let user_ata_lp = next_account_info(accounts_iter)?;
    let user_ata_a = next_account_info(accounts_iter)?;
    let user_ata_b = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;

    if !user.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if amount_lp_in == 0 {
        return Err(AmmError::ZeroLiquidityAmount.into());
    }

    let mut pool_data
        = try_from_slice_unchecked::<LiquidityPool>(&pool.data.borrow())?;

    let (mint_lo, mint_hi) = if pool_data.mint_a < pool_data.mint_b {
        (pool_data.mint_a, pool_data.mint_b)
    } else {
        (pool_data.mint_b, pool_data.mint_a)
    };

    let expected_pool = Pubkey::create_program_address(
        &[b"pool", mint_lo.as_ref(), mint_hi.as_ref(), &pool_data.fee_bps.to_le_bytes(), &[pool_data.bump]], 
        program_id,
    ).map_err(|_| ProgramError::InvalidSeeds)?;

    msg!("passed pool: {}", pool.key);
    msg!("expected pool: {}", expected_pool);

    if expected_pool != *pool.key {
        return Err(AmmError::PoolAddressMismatch.into());
    }

    if *mint_a.key != pool_data.mint_a {
        return Err(AmmError::MintAddressMismatch.into());
    }

    if *mint_b.key != pool_data.mint_b {
        return Err(AmmError::MintAddressMismatch.into());
    }
    
    if *vault_a.key != get_associated_token_address(pool.key, mint_a.key) {
        return Err(AmmError::VaultAddressMismatch.into());
    }

    if *vault_b.key != get_associated_token_address(pool.key, mint_b.key) {
        return Err(AmmError::VaultAddressMismatch.into());
    }

    let (expected_lp_mint, _lp_mint_bump) = Pubkey::find_program_address(
    &[b"lp_mint", pool.key.as_ref()], program_id);

    if *mint_lp.key != expected_lp_mint {
        return Err(AmmError::LpMintAddressMismatch.into());
    }

    // compute withdrawal amounts
    let mint_lp_data = 
        Mint::unpack(&mint_lp.data.borrow())?;

    let total_lp = mint_lp_data.supply as u128;
    let amount_lp_in  = amount_lp_in as u128;
    let reserve_a = pool_data.reserve_a as u128;
    let reserve_b = pool_data.reserve_b as u128;

    if total_lp == 0 {
        return Err(ProgramError::UninitializedAccount);
    }

    let a_out = amount_lp_in.checked_mul(reserve_a)
        .ok_or(ProgramError::ArithmeticOverflow)? / total_lp;
    let b_out = amount_lp_in.checked_mul(reserve_b)
        .ok_or(ProgramError::ArithmeticOverflow)? / total_lp;

    if a_out < amount_a_min as u128 || b_out < amount_b_min as u128 {
        return Err(AmmError::SlippageExceed.into());
    }

    // burn lp tokens from user_ata_lp
    invoke(
        &burn(
            token_program.key, 
            user_ata_lp.key, 
            mint_lp.key, 
            user.key, 
            &[], 
            amount_lp_in as u64,
        )?, 
        &[user_ata_lp.clone(), mint_lp.clone(), user.clone()],
    )?;

    let a_out = a_out as u64;
    let b_out = b_out as u64;

    let mint_a_data = 
        Mint::unpack(&mint_a.data.borrow())?;
    let mint_b_data =
        Mint::unpack(&mint_b.data.borrow())?;

    // transfer a_out from vault_a to user_ata_a
    invoke_signed(
        &transfer_checked(
            token_program.key, 
            vault_a.key, 
            mint_a.key, 
            user_ata_a.key, 
            pool.key, 
            &[], 
            a_out, 
            mint_a_data.decimals,
        )?, 
        &[vault_a.clone(), mint_a.clone(), user_ata_a.clone(), pool.clone()], 
        &[
            &[b"pool", mint_lo.as_ref(), mint_hi.as_ref(), &pool_data.fee_bps.to_le_bytes(), &[pool_data.bump]]
        ],
    )?;

    // transfer b_out from vault_b to user_ata_b
    invoke_signed(
        &transfer_checked(
            token_program.key, 
            vault_b.key, 
            mint_b.key, 
            user_ata_b.key, 
            pool.key, 
            &[], 
            b_out, 
            mint_b_data.decimals,
        )?, 
        &[vault_b.clone(), mint_b.clone(), user_ata_b.clone(), pool.clone()], 
        &[
            &[b"pool", mint_lo.as_ref(), mint_hi.as_ref(), &pool_data.fee_bps.to_le_bytes(), &[pool_data.bump]]
        ],
    )?;

    pool_data.reserve_a = pool_data.reserve_a.checked_sub(a_out)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    pool_data.reserve_b = pool_data.reserve_b.checked_sub(b_out)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    pool_data.serialize(&mut &mut pool.data.borrow_mut()[..])?;

    Ok(())
}

pub fn process_swap(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount_in: u64,
    min_out: u64,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let user = next_account_info(accounts_iter)?;
    let pool = next_account_info(accounts_iter)?;
    let mint_in = next_account_info(accounts_iter)?;
    let mint_out = next_account_info(accounts_iter)?;
    let vault_in = next_account_info(accounts_iter)?;
    let vault_out = next_account_info(accounts_iter)?;
    let user_ata_in = next_account_info(accounts_iter)?;
    let user_ata_out = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;
    let associated_token_program = next_account_info(accounts_iter)?;

    if !user.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if amount_in == 0 {
        return Err(AmmError::ZeroSwapAmount.into());
    }

    let mut pool_data = 
        try_from_slice_unchecked::<LiquidityPool>(&pool.data.borrow())?;

    let (mint_lo, mint_hi) = if mint_in.key < mint_out.key {
        (mint_in.key.clone(), mint_out.key.clone())
    } else {
        (mint_out.key.clone(), mint_in.key.clone())
    };

    let expected_pool = Pubkey::create_program_address(
        &[b"pool", mint_lo.as_ref(), mint_hi.as_ref(), &pool_data.fee_bps.to_le_bytes(), &[pool_data.bump]], 
        program_id,
    ).map_err(|_| ProgramError::InvalidSeeds)?;    

    if expected_pool != *pool.key {
        return Err(AmmError::PoolAddressMismatch.into());
    }

    let reserve_in;
    let reserve_out;

    if *mint_in.key == pool_data.mint_a {
        reserve_in = pool_data.reserve_a as u128;
        reserve_out = pool_data.reserve_b as u128;
    }
    else {
        reserve_in = pool_data.reserve_b as u128;
        reserve_out = pool_data.reserve_a as u128;
    }

    let fee_bps = pool_data.fee_bps as u128;

    let amount_in_post_fee= 
        (amount_in as u128) * (10_000 - fee_bps);

    let amount_out= 
        ((reserve_out * amount_in_post_fee) / (reserve_in * 10_000 + amount_in_post_fee)) 
        as u64;

    if amount_out < min_out {
        return Err(AmmError::SlippageExceed.into());
    }

    let mint_in_decimals = Mint::unpack(&mint_in.data.borrow())?.decimals;

    // transfer amount_in of mint_in from user_ata_in to vault_in
    invoke(
        &transfer_checked(
            token_program.key,
            user_ata_in.key, 
            mint_in.key, 
            vault_in.key, 
            user.key, 
            &[], 
            amount_in, 
            mint_in_decimals,
        )?, 
        &[user_ata_in.clone(), mint_in.clone(), vault_in.clone(), user.clone()], 
    )?;

    let mint_out_decimals = Mint::unpack(&mint_out.data.borrow())?.decimals;

    // transfer amount_out of mint_out from vault_out to user_ata_out
    invoke_signed(
        &transfer_checked(
            token_program.key, 
            vault_out.key, 
            mint_out.key, 
            user_ata_out.key, 
            pool.key, 
            &[], 
            amount_out, 
            mint_out_decimals,
        )?, 
        &[vault_out.clone(), mint_out.clone(), user_ata_out.clone(), pool.clone()], 
        &[
            &[b"pool", mint_lo.as_ref(), mint_hi.as_ref(), &pool_data.fee_bps.to_le_bytes(), &[pool_data.bump]]
        ],
    )?;

    if *mint_in.key == pool_data.mint_a {
        pool_data.reserve_a += amount_in;
        pool_data.reserve_b -= amount_out;
    }
    else {
        pool_data.reserve_a -= amount_out;
        pool_data.reserve_b += amount_in;
    }

    pool_data.serialize(&mut &mut pool.data.borrow_mut()[..])?;

    Ok(())
}