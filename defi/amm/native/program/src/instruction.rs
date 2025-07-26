use solana_program::program_error::ProgramError;

use borsh::BorshDeserialize;

pub enum AmmInstruction {
    CreatePool {
        amount_a: u64,
        amount_b: u64,
        fee_bps: u16,
    },
    ProvideLiquidity {
        amount_a_desired: u64,
        amount_b_desired: u64,
        amount_a_min: u64,
        amount_b_min: u64,
    },
    WithdrawLiquidity {
        amount_lp_in: u64,
        amount_a_min: u64,
        amount_b_min: u64,
    },
    Swap {
        amount_in: u64,
        min_out: u64,
    },

}

impl AmmInstruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&discriminator, rest) = input.split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;
    
        Ok(
            match discriminator {
                0 => {
                    let payload = CreatePoolPayload::try_from_slice(rest)
                        .map_err(|_| ProgramError::InvalidInstructionData)?;

                    Self::CreatePool { 
                        amount_a: payload.amount_a, 
                        amount_b: payload.amount_b,
                        fee_bps: payload.fee_bps,
                    }
                },
                1 => {
                    let payload = ProvideLiquidityPayload::try_from_slice(rest)
                        .map_err(|_| ProgramError::InvalidInstructionData)?;

                    Self::ProvideLiquidity { 
                        amount_a_desired: payload.amount_a_desired,
                        amount_b_desired: payload.amount_b_desired,
                        amount_a_min: payload.amount_a_min,
                        amount_b_min: payload.amount_b_min,
                    }
                },
                2 => {
                    let payload = WithdrawLiquidityPayload::try_from_slice(rest)
                        .map_err(|_| ProgramError::InvalidInstructionData)?;

                    Self::WithdrawLiquidity { 
                        amount_lp_in: payload.amount_lp_in, 
                        amount_a_min: payload.amount_a_min, 
                        amount_b_min: payload.amount_b_min,
                    }
                },
                3 => {
                    let payload = SwapPayload::try_from_slice(rest)
                        .map_err(|_| ProgramError::InvalidInstructionData)?;

                    Self::Swap { 
                        amount_in: payload.amount_in,
                        min_out: payload.min_out, 
                    }
                },

                _ => return Err(ProgramError::InvalidInstructionData)
            }
        )
    }
}

#[derive(BorshDeserialize)]
struct CreatePoolPayload {
    amount_a: u64,
    amount_b: u64,
    fee_bps: u16,
}

#[derive(BorshDeserialize)]
struct ProvideLiquidityPayload {
    amount_a_desired: u64,
    amount_b_desired: u64,
    amount_a_min: u64,
    amount_b_min: u64,
}

#[derive(BorshDeserialize)]
struct WithdrawLiquidityPayload {
    amount_lp_in: u64,
    amount_a_min: u64,
    amount_b_min: u64,
}

#[derive(BorshDeserialize)]
struct SwapPayload {
    amount_in: u64,
    min_out: u64,
}