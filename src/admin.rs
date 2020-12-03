//! Module for processing admin-only instructions.

#![cfg(feature = "program")]

use crate::{
    error::SwapError,
    fees::Fees,
    instruction::{AdminInstruction, RampAData},
};
#[cfg(target_arch = "bpf")]
use solana_sdk::info;
use solana_sdk::{account_info::AccountInfo, entrypoint::ProgramResult, info, pubkey::Pubkey};

/// Process admin instruction
pub fn process_admin_instruction(
    instruction: &AdminInstruction,
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    match *instruction {
        // AdminInstruction::RampA(RampAData {
        //     future_amp,
        //     future_time,
        // }) => {
        //     info!("Instruction : Ramp A");
        //     ramp_a(future_amp, future_time, accounts)
        // }
        // AdminInstruction::StopRampA() => {
        //     info!("Instruction: Stop Ramp A");
        //     stop_ramp_a(accounts)
        // }
        // AdminInstruction::Pause() => {
        //     info!("Instruction: Pause");
        //     pause(accounts)
        // }
        _ => Err(SwapError::InvalidInstruction.into()),
    }
}

/// Access control for admin only instructions
fn is_admin(_expected_admin_key: &Pubkey, _admin_account_info: &AccountInfo) -> ProgramResult {
    unimplemented!("is_admin not implemented")
}

/// Ramp to future a
fn ramp_a(_future_a: u64, _future_time: u64, _account: &[AccountInfo]) -> ProgramResult {
    unimplemented!("ramp_a not implemented");
}

/// Stop ramp a
fn stop_ramp_a(_accounts: &[AccountInfo]) -> ProgramResult {
    unimplemented!("stop_ramp_a not implemented");
}

/// Pause swap
fn pause(_accounts: &[AccountInfo]) -> ProgramResult {
    unimplemented!("pause not implemented")
}

/// Unpause swap
fn unpause(_accounts: &[AccountInfo]) -> ProgramResult {
    unimplemented!("unpause not implemented")
}

/// Set fee account a
fn set_fee_account_a(_accounts: &[AccountInfo], _new_fee_account_a: &Pubkey) -> ProgramResult {
    unimplemented!("set_fee_account_a not implemented")
}

/// Set fee account a
fn set_fee_account_b(_accounts: &[AccountInfo], _new_fee_account_b: &Pubkey) -> ProgramResult {
    unimplemented!("set_fee_account_b not implemented")
}

/// Apply new admin
fn apply_new_admin(_accounts: &[AccountInfo], _new_admin: &Pubkey) -> ProgramResult {
    unimplemented!("apply_new_admin not implemented");
}

/// Commit new admin
fn commit_new_admin(_accounts: &[AccountInfo], _new_admin: &Pubkey) -> ProgramResult {
    unimplemented!("set_new_admin not implemented");
}

/// Revert new admin
fn revert_new_admin(_accounts: &[AccountInfo], _new_admin: &Pubkey) -> ProgramResult {
    unimplemented!("revert_new_admin not implemented");
}

/// Apply new fees
fn apply_new_fees(_accounts: &[AccountInfo]) -> ProgramResult {
    unimplemented!("apply_new_fees not implemented");
}

/// Commit new fees
fn commit_new_fees(_accounts: &[AccountInfo], _new_fees: Fees) -> ProgramResult {
    unimplemented!("set_new_fees not implemented");
}

/// Revert new fees
fn revert_new_fees(_accounts: &[AccountInfo]) -> ProgramResult {
    unimplemented!("set_new_fees not implemented");
}
