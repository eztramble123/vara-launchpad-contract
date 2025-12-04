//! Token Launchpad Contract - WASM entry point.

#![no_std]

#[cfg(target_arch = "wasm32")]
pub use launchpad_app::LaunchpadProgram;
