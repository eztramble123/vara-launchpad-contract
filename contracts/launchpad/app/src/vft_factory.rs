//! VFT Token Factory - Deploys new VFT tokens for launches.
//!
//! This module handles the deployment of new VFT token contracts
//! for fair token launches.

use alloc::string::String;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sails_rs::prelude::*;
use gstd::prog::ProgramGenerator;
use vara_contracts_shared::{ContractError};
use crate::vft_client::U256;

/// VFT token initialization parameters.
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub struct VftInitParams {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: U256,
    pub initial_owner: ActorId,
}

/// VFT Factory for deploying token contracts.
pub struct VftFactory;

impl VftFactory {
    /// Deploy a new VFT token contract.
    ///
    /// This deploys a standard VFT token with:
    /// - Fixed total supply minted to the launchpad
    /// - Standard 18 decimals
    /// - Transfer/approval capabilities
    /// - Configurable gas for program creation and reply handling
    pub async fn deploy_token(
        name: String,
        symbol: String,
        total_supply: U256,
        code_id: CodeId,
        gas_for_program: u64,
        gas_for_reply: u64,
    ) -> Result<ActorId, ContractError> {
        // Get launchpad's address (tokens will be minted here)
        let launchpad_address = gstd::exec::program_id();

        // Prepare initialization parameters
        let init_params = VftInitParams {
            name,
            symbol,
            decimals: 18,  // Standard decimals
            total_supply,
            initial_owner: launchpad_address,  // Mint all tokens to launchpad
        };

        // Encode init params
        let payload = init_params.encode();

        // Deploy the token contract using ProgramGenerator with configurable gas
        // ProgramGenerator handles salt generation automatically
        let (_, token_address) = ProgramGenerator::create_program_bytes_with_gas(
            code_id,
            payload,
            gas_for_program,
            0,  // No value transfer
        )
        .map_err(|_| ContractError::invalid_state("Failed to deploy token"))?;

        // Send a ping to verify the program is ready and wait for reply
        // using the configured reply gas
        gstd::msg::send_bytes_with_gas_for_reply(token_address, b"", gas_for_reply, 0, 0)
            .map_err(|_| ContractError::TransferFailed)?
            .await
            .map_err(|_| ContractError::invalid_state("Token deployment verification failed"))?;

        Ok(token_address)
    }
    
    /// Deploy a token with custom decimals.
    pub async fn deploy_token_with_decimals(
        name: String,
        symbol: String,
        total_supply: U256,
        decimals: u8,
        code_id: CodeId,
        gas_for_program: u64,
        gas_for_reply: u64,
    ) -> Result<ActorId, ContractError> {
        let launchpad_address = gstd::exec::program_id();

        let init_params = VftInitParams {
            name,
            symbol,
            decimals,
            total_supply,
            initial_owner: launchpad_address,
        };

        let payload = init_params.encode();

        // Deploy using ProgramGenerator with configurable gas
        // ProgramGenerator handles salt generation automatically
        let (_, token_address) = ProgramGenerator::create_program_bytes_with_gas(
            code_id,
            payload,
            gas_for_program,
            0,
        )
        .map_err(|_| ContractError::invalid_state("Failed to deploy token"))?;

        // Send a ping to verify the program is ready and wait for reply
        // using the configured reply gas
        gstd::msg::send_bytes_with_gas_for_reply(token_address, b"", gas_for_reply, 0, 0)
            .map_err(|_| ContractError::TransferFailed)?
            .await
            .map_err(|_| ContractError::invalid_state("Token deployment verification failed"))?;

        Ok(token_address)
    }
}

/// Helper to calculate token amounts with decimals.
pub fn calculate_token_amount(amount: u128, decimals: u8) -> U256 {
    let multiplier = 10u128.pow(decimals as u32);
    U256::from(amount) * U256::from(multiplier)
}

/// Standard token configuration for fair launches.
pub struct TokenConfig;

impl TokenConfig {
    /// Standard token supply: 1 billion tokens.
    pub const STANDARD_SUPPLY: u128 = 1_000_000_000;
    
    /// Standard decimals.
    pub const STANDARD_DECIMALS: u8 = 18;
    
    /// Get standard total supply with decimals.
    pub fn standard_supply_with_decimals() -> U256 {
        calculate_token_amount(Self::STANDARD_SUPPLY, Self::STANDARD_DECIMALS)
    }
}