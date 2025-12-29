//! VFT (Vara Fungible Token) client for interacting with token contracts.
//!
//! Provides async messaging interface for VFT standard operations.

use alloc::vec::Vec;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sails_rs::prelude::*;
use vara_contracts_shared::{Amount, ContractError};

pub type U256 = u128;

// =============================================================================
// VFT MESSAGE TYPES
// =============================================================================

/// VFT action messages for token operations.
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub enum VftAction {
    Transfer { to: ActorId, value: U256 },
    TransferFrom { from: ActorId, to: ActorId, value: U256 },
    Approve { spender: ActorId, value: U256 },
    Mint { to: ActorId, value: U256 },
    Burn { from: ActorId, value: U256 },
}

/// VFT query messages for reading token state.
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub enum VftQuery {
    BalanceOf { account: ActorId },
    Allowance { owner: ActorId, spender: ActorId },
    TotalSupply,
    Name,
    Symbol,
    Decimals,
}

/// VFT events emitted by token contracts.
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub enum VftEvent {
    Transfer { from: ActorId, to: ActorId, value: U256 },
    Approval { owner: ActorId, spender: ActorId, value: U256 },
    Mint { to: ActorId, value: U256 },
    Burn { from: ActorId, value: U256 },
}

/// Token metadata for DEX compatibility.
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub struct TokenMetadata {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: U256,
}

/// Token holder information for bridge systems.
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub struct TokenHolder {
    pub address: ActorId,
    pub balance: Amount,
    pub claimed: Amount,
}

/// Launch token information for external systems.
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub struct LaunchTokenInfo {
    pub token_address: ActorId,
    pub total_supply: Amount,
    pub circulating_supply: Amount,
    pub price_per_token: Amount,
    pub launch_ended: bool,
}

// =============================================================================
// VFT CLIENT
// =============================================================================

/// VFT client for async token operations.
pub struct VftClient;

impl VftClient {
    /// Send an async message to VFT contract for actions.
    pub async fn send_action(
        token_address: ActorId,
        action: VftAction,
    ) -> Result<(), ContractError> {
        let payload = action.encode();
        
        gstd::msg::send_bytes_for_reply(token_address, payload, 0, 0)
            .map_err(|_| ContractError::TransferFailed)?
            .await
            .map_err(|_| ContractError::TransferFailed)?;
            
        Ok(())
    }
    
    /// Send an async query to VFT contract.
    pub async fn send_query(
        token_address: ActorId,
        query: VftQuery,
    ) -> Result<Vec<u8>, ContractError> {
        let payload = query.encode();
        
        let response = gstd::msg::send_bytes_for_reply(token_address, payload, 0, 0)
            .map_err(|_| ContractError::TransferFailed)?
            .await
            .map_err(|_| ContractError::TransferFailed)?;
            
        Ok(response)
    }
    
    /// Transfer tokens from the contract to a recipient.
    pub async fn transfer(
        token_address: ActorId,
        to: ActorId,
        amount: U256,
    ) -> Result<(), ContractError> {
        Self::send_action(
            token_address,
            VftAction::Transfer { to, value: amount },
        ).await
    }
    
    /// Transfer tokens on behalf of another account (requires approval).
    pub async fn transfer_from(
        token_address: ActorId,
        from: ActorId,
        to: ActorId,
        amount: U256,
    ) -> Result<(), ContractError> {
        Self::send_action(
            token_address,
            VftAction::TransferFrom { from, to, value: amount },
        ).await
    }
    
    /// Approve another account to spend tokens.
    pub async fn approve(
        token_address: ActorId,
        spender: ActorId,
        amount: U256,
    ) -> Result<(), ContractError> {
        Self::send_action(
            token_address,
            VftAction::Approve { spender, value: amount },
        ).await
    }
    
    /// Query token balance of an account.
    pub async fn balance_of(
        token_address: ActorId,
        account: ActorId,
    ) -> Result<U256, ContractError> {
        let response = Self::send_query(
            token_address,
            VftQuery::BalanceOf { account },
        ).await?;
        
        U256::decode(&mut response.as_slice())
            .map_err(|_| ContractError::TransferFailed)
    }
    
    /// Query spending allowance.
    pub async fn allowance(
        token_address: ActorId,
        owner: ActorId,
        spender: ActorId,
    ) -> Result<U256, ContractError> {
        let response = Self::send_query(
            token_address,
            VftQuery::Allowance { owner, spender },
        ).await?;
        
        U256::decode(&mut response.as_slice())
            .map_err(|_| ContractError::TransferFailed)
    }
    
    /// Query total token supply.
    pub async fn total_supply(token_address: ActorId) -> Result<U256, ContractError> {
        let response = Self::send_query(token_address, VftQuery::TotalSupply).await?;
        
        U256::decode(&mut response.as_slice())
            .map_err(|_| ContractError::TransferFailed)
    }
    
    /// Query token metadata for DEX listing.
    pub async fn get_metadata(token_address: ActorId) -> Result<TokenMetadata, ContractError> {
        use alloc::string::String;
        
        let name_response = Self::send_query(token_address, VftQuery::Name).await?;
        let symbol_response = Self::send_query(token_address, VftQuery::Symbol).await?;
        let decimals_response = Self::send_query(token_address, VftQuery::Decimals).await?;
        let supply_response = Self::send_query(token_address, VftQuery::TotalSupply).await?;
        
        let name = String::decode(&mut name_response.as_slice())
            .map_err(|_| ContractError::TransferFailed)?;
        let symbol = String::decode(&mut symbol_response.as_slice())
            .map_err(|_| ContractError::TransferFailed)?;
        let decimals = u8::decode(&mut decimals_response.as_slice())
            .map_err(|_| ContractError::TransferFailed)?;
        let total_supply = U256::decode(&mut supply_response.as_slice())
            .map_err(|_| ContractError::TransferFailed)?;
        
        Ok(TokenMetadata {
            name,
            symbol,
            decimals,
            total_supply,
        })
    }
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Check if the launchpad contract has sufficient token balance.
pub async fn verify_token_balance(
    token_address: ActorId,
    required_amount: U256,
) -> Result<bool, ContractError> {
    let contract_address = gstd::exec::program_id();
    let balance = VftClient::balance_of(token_address, contract_address).await?;
    Ok(balance >= required_amount)
}

/// Check if creator has approved launchpad to transfer tokens.
pub async fn verify_token_approval(
    token_address: ActorId,
    owner: ActorId,
    required_amount: U256,
) -> Result<bool, ContractError> {
    let contract_address = gstd::exec::program_id();
    let allowance = VftClient::allowance(token_address, owner, contract_address).await?;
    Ok(allowance >= required_amount)
}