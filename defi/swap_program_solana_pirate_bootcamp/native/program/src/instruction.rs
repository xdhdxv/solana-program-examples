use solana_program::program_error::ProgramError;

use borsh::BorshDeserialize;

pub enum SwapInstruction {
    CreatePool,
    FundPool {
        amount: u64,
    },
    Swap {
        amount_to_swap: u64,
    }
}

impl SwapInstruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&discriminator, rest) = input.split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;

        Ok(
            match discriminator {
                0 => {
                    Self::CreatePool
                },
                1 => {
                    let payload = FundPayload::try_from_slice(rest)
                        .map_err(|_| ProgramError::InvalidInstructionData)?;

                    Self::FundPool { 
                        amount: payload.amount 
                    }
                },
                2 => {
                    let payload = SwapPayload::try_from_slice(rest)
                        .map_err(|_| ProgramError::InvalidInstructionData)?;

                    Self::Swap { 
                        amount_to_swap: payload.amount_to_swap 
                    }
                },

                _ => return Err(ProgramError::InvalidInstructionData)
            }
        )
    } 
}

#[derive(BorshDeserialize)]
struct FundPayload {
    amount: u64,
}

#[derive(BorshDeserialize)]
struct SwapPayload {
    amount_to_swap: u64,
}