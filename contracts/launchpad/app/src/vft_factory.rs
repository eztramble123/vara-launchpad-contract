//! VFT Token Factory - Deploys new VFT tokens for launches.
//!
//! This module handles the deployment of new VFT token contracts
//! for Pump.fun-style fair launches.

use alloc::string::String;
use alloc::vec::Vec;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sails_rs::prelude::*;
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
    pub async fn deploy_token(
        name: String,
        symbol: String,
        total_supply: U256,
        code_id: CodeId,  // Code ID of uploaded VFT contract code
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
        
        // Deploy the token contract
        // Note: In production, you'd use gstd::prog::create_program_with_gas
        // to deploy from code_id with the init payload
        let (message_id, token_address) = gstd::prog::create_program_with_gas(
            code_id,
            payload,
            0,  // No value transfer
            10_000_000_000,  // Gas for deployment
            0,  // No reply deposit
        )
        .map_err(|_| ContractError::invalid_state("Failed to deploy token"))?;
        
        // Wait for deployment confirmation
        gstd::msg::send_bytes_for_reply(token_address, b"ping", 0, 0)
            .map_err(|_| ContractError::TransferFailed)?
            .await
            .map_err(|_| ContractError::invalid_state("Token deployment confirmation failed"))?;
        
        Ok(token_address)
    }
    
    /// Deploy a token with custom decimals.
    pub async fn deploy_token_with_decimals(
        name: String,
        symbol: String,
        total_supply: U256,
        decimals: u8,
        code_id: CodeId,
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
        
        let (message_id, token_address) = gstd::prog::create_program_with_gas(
            code_id,
            payload,
            0,
            10_000_000_000,
            0,
        )
        .map_err(|_| ContractError::invalid_state("Failed to deploy token"))?;
        
        // Verify deployment
        gstd::msg::send_bytes_for_reply(token_address, b"ping", 0, 0)
            .map_err(|_| ContractError::TransferFailed)?
            .await
            .map_err(|_| ContractError::invalid_state("Token deployment confirmation failed"))?;
        
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