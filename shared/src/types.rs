//! Common types used across contracts.

use alloc::string::String;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sails_rs::prelude::*;

/// Unique identifier for deals, campaigns, loans, etc.
pub type Id = u64;

/// Amount type for token values (supports up to 10^38).
pub type Amount = u128;

/// Block number type for time-based operations.
pub type BlockNumber = u32;

/// Basis points (1/100th of a percent, so 10000 = 100%).
pub type BasisPoints = u16;

/// Maximum basis points (100%).
pub const MAX_BASIS_POINTS: BasisPoints = 10_000;

/// Role identifier for access control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Encode, Decode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub struct RoleId(pub [u8; 32]);

impl RoleId {
    /// Default admin role - has permission to manage other roles.
    pub const DEFAULT_ADMIN: Self = Self([0u8; 32]);

    /// Create a new role from a string identifier.
    pub fn from_str(s: &str) -> Self {
        let mut bytes = [0u8; 32];
        let s_bytes = s.as_bytes();
        let len = s_bytes.len().min(32);
        bytes[..len].copy_from_slice(&s_bytes[..len]);
        Self(bytes)
    }
}

impl Default for RoleId {
    fn default() -> Self {
        Self::DEFAULT_ADMIN
    }
}

/// Milestone for escrow and crowdfunding contracts.
#[derive(Debug, Clone, Encode, Decode, TypeInfo, PartialEq, Eq)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub struct Milestone {
    /// Description of the milestone.
    pub description: String,
    /// Amount to be released when milestone is completed.
    pub amount: Amount,
    /// Whether the milestone has been completed.
    pub completed: bool,
}

impl Milestone {
    pub fn new(description: String, amount: Amount) -> Self {
        Self {
            description,
            amount,
            completed: false,
        }
    }
}

/// Vote choice in governance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub enum VoteChoice {
    /// Vote in favor.
    For,
    /// Vote against.
    Against,
    /// Abstain from voting.
    Abstain,
}

/// Status of a time-sensitive operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, TypeInfo, Default)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub enum Status {
    /// Initial state, not yet active.
    #[default]
    Pending,
    /// Currently active.
    Active,
    /// Successfully completed.
    Completed,
    /// Cancelled by authorized party.
    Cancelled,
    /// Failed to meet conditions.
    Failed,
}

/// Token type for contracts supporting both native and VFT tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub enum TokenType {
    /// Native VARA token.
    Native,
    /// VFT (Vara Fungible Token) contract address.
    Vft(ActorId),
}

impl Default for TokenType {
    fn default() -> Self {
        Self::Native
    }
}

/// Configuration for vesting schedules.
#[derive(Debug, Clone, Encode, Decode, TypeInfo, PartialEq, Eq)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub struct VestingConfig {
    /// Block when vesting starts.
    pub start_block: BlockNumber,
    /// Duration of cliff period in blocks (no tokens released).
    pub cliff_duration: BlockNumber,
    /// Total vesting duration in blocks (including cliff).
    pub vesting_duration: BlockNumber,
}

impl VestingConfig {
    pub fn new(
        start_block: BlockNumber,
        cliff_duration: BlockNumber,
        vesting_duration: BlockNumber,
    ) -> Self {
        Self {
            start_block,
            cliff_duration,
            vesting_duration,
        }
    }

    /// Returns the block when the cliff ends.
    pub fn cliff_end(&self) -> BlockNumber {
        self.start_block.saturating_add(self.cliff_duration)
    }

    /// Returns the block when vesting ends.
    pub fn vesting_end(&self) -> BlockNumber {
        self.start_block.saturating_add(self.vesting_duration)
    }
}
