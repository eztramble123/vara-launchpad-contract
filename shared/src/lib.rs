//! Shared utilities for Vara contract templates.
//!
//! This crate provides common types, error handling, and utility functions
//! used across all contract templates in the library.

#![no_std]

extern crate alloc;

pub mod errors;
pub mod types;

pub use errors::*;
pub use types::*;
