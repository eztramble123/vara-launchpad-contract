//! Common error types used across contracts.

use alloc::string::String;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;

/// Common errors that can occur across multiple contracts.
#[derive(Debug, Clone, Encode, Decode, TypeInfo, PartialEq, Eq)]
pub enum ContractError {
    /// Caller is not authorized to perform this action.
    Unauthorized,
    /// The requested item was not found.
    NotFound,
    /// The item already exists.
    AlreadyExists,
    /// Invalid input parameter.
    InvalidInput(String),
    /// Insufficient balance or funds.
    InsufficientFunds,
    /// Operation is not allowed in the current state.
    InvalidState(String),
    /// Deadline has passed.
    DeadlinePassed,
    /// Deadline has not passed yet.
    DeadlineNotPassed,
    /// Amount exceeds the maximum allowed.
    AmountExceedsMax,
    /// Amount is below the minimum required.
    AmountBelowMin,
    /// The operation has already been performed.
    AlreadyProcessed,
    /// Transfer failed.
    TransferFailed,
    /// Arithmetic overflow.
    Overflow,
    /// Zero amount not allowed.
    ZeroAmount,
    /// Address is zero/invalid.
    ZeroAddress,
}

impl ContractError {
    pub fn invalid_input(msg: &str) -> Self {
        Self::InvalidInput(String::from(msg))
    }

    pub fn invalid_state(msg: &str) -> Self {
        Self::InvalidState(String::from(msg))
    }
}

/// Result type alias using ContractError.
pub type ContractResult<T> = Result<T, ContractError>;
