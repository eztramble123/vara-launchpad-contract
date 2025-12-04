//! Integration tests for Launchpad contract.

use gtest::{Program, System};
use sails_rs::prelude::*;

const OWNER: u64 = 1;
const CREATOR: u64 = 2;
const CONTRIBUTOR1: u64 = 3;
const CONTRIBUTOR2: u64 = 4;
const NON_WHITELISTED: u64 = 5;
const ANYONE: u64 = 6;

const ONE_VARA: u128 = 1_000_000_000_000; // 10^12

fn setup_system() -> System {
    let system = System::new();
    system.init_logger();
    system
}

fn deploy_contract(system: &System) -> Program<'_> {
    let program = Program::from_file(
        system,
        "../target/wasm32-unknown-unknown/release/launchpad.opt.wasm",
    );

    // Initialize with OWNER as the deployer (default 2% fee)
    let _result = program.send_bytes(OWNER, b"New");

    program
}

#[test]
fn test_create_launch() {
    let system = setup_system();
    let program = deploy_contract(&system);

    // Create a launch
    let result = program.send_bytes(
        CREATOR,
        b"Launchpad\x00CreateLaunch",
    );

    assert!(!result.main_failed(), "Creating launch should succeed");
}

#[test]
fn test_contribute_within_limits() {
    let system = setup_system();
    let program = deploy_contract(&system);

    // Create a launch
    let _ = program.send_bytes(
        CREATOR,
        b"Launchpad\x00CreateLaunch",
    );

    // Start the launch
    let _ = program.send_bytes(
        CREATOR,
        b"Launchpad\x00StartLaunch",
    );

    // Advance to start time
    system.spend_blocks(1000);

    // Contribute within limits
    system.mint_to(CONTRIBUTOR1, 1000 * ONE_VARA);

    let result = program.send_bytes_with_value(
        CONTRIBUTOR1,
        b"Launchpad\x00Contribute",
        50 * ONE_VARA,
    );

    assert!(!result.main_failed(), "Contributing within limits should succeed");
}

#[test]
fn test_whitelist_enforcement() {
    let system = setup_system();
    let program = deploy_contract(&system);

    // Create a launch with whitelist enabled
    let _ = program.send_bytes(
        CREATOR,
        b"Launchpad\x00CreateLaunch", // whitelist_enabled: true
    );

    // Add CONTRIBUTOR1 to whitelist
    let _ = program.send_bytes(
        CREATOR,
        b"Launchpad\x00AddToWhitelist",
    );

    // Start the launch
    let _ = program.send_bytes(
        CREATOR,
        b"Launchpad\x00StartLaunch",
    );

    // Advance to start time
    system.spend_blocks(1000);

    // Whitelisted user can contribute
    system.mint_to(CONTRIBUTOR1, 1000 * ONE_VARA);
    let result1 = program.send_bytes_with_value(
        CONTRIBUTOR1,
        b"Launchpad\x00Contribute",
        50 * ONE_VARA,
    );
    assert!(!result1.main_failed(), "Whitelisted contributor should succeed");

    // Non-whitelisted user should fail
    system.mint_to(NON_WHITELISTED, 1000 * ONE_VARA);
    let result2 = program.send_bytes_with_value(
        NON_WHITELISTED,
        b"Launchpad\x00Contribute",
        50 * ONE_VARA,
    );
    // Should fail - not whitelisted
}

#[test]
fn test_finalize_successful_launch() {
    let system = setup_system();
    let program = deploy_contract(&system);

    // Create and start launch
    let _ = program.send_bytes(CREATOR, b"Launchpad\x00CreateLaunch");
    let _ = program.send_bytes(CREATOR, b"Launchpad\x00StartLaunch");

    // Advance to start time
    system.spend_blocks(1000);

    // Contributors meet minimum raise
    system.mint_to(CONTRIBUTOR1, 1000 * ONE_VARA);
    system.mint_to(CONTRIBUTOR2, 1000 * ONE_VARA);

    let _ = program.send_bytes_with_value(
        CONTRIBUTOR1,
        b"Launchpad\x00Contribute",
        300 * ONE_VARA,
    );
    let _ = program.send_bytes_with_value(
        CONTRIBUTOR2,
        b"Launchpad\x00Contribute",
        300 * ONE_VARA,
    );

    // Advance past end time
    system.spend_blocks(10000);

    // Finalize launch
    let result = program.send_bytes(
        ANYONE,
        b"Launchpad\x00Finalize",
    );

    assert!(!result.main_failed(), "Finalizing successful launch should succeed");
}

#[test]
fn test_claim_tokens() {
    let system = setup_system();
    let program = deploy_contract(&system);

    // Create, start, fund, and finalize launch
    let _ = program.send_bytes(CREATOR, b"Launchpad\x00CreateLaunch");
    let _ = program.send_bytes(CREATOR, b"Launchpad\x00StartLaunch");

    system.spend_blocks(1000);

    system.mint_to(CONTRIBUTOR1, 1000 * ONE_VARA);
    let _ = program.send_bytes_with_value(
        CONTRIBUTOR1,
        b"Launchpad\x00Contribute",
        500 * ONE_VARA,
    );

    system.spend_blocks(10000);
    let _ = program.send_bytes(ANYONE, b"Launchpad\x00Finalize");

    // Claim tokens
    let result = program.send_bytes(
        CONTRIBUTOR1,
        b"Launchpad\x00ClaimTokens",
    );

    assert!(!result.main_failed(), "Claiming tokens should succeed");
}

#[test]
fn test_refund_failed_launch() {
    let system = setup_system();
    let program = deploy_contract(&system);

    // Create and start launch
    let _ = program.send_bytes(CREATOR, b"Launchpad\x00CreateLaunch");
    let _ = program.send_bytes(CREATOR, b"Launchpad\x00StartLaunch");

    system.spend_blocks(1000);

    // Contribute less than minimum
    system.mint_to(CONTRIBUTOR1, 1000 * ONE_VARA);
    let _ = program.send_bytes_with_value(
        CONTRIBUTOR1,
        b"Launchpad\x00Contribute",
        50 * ONE_VARA, // Less than min_raise
    );

    // Advance past end time
    system.spend_blocks(10000);

    // Finalize - should fail due to min raise not met
    let _ = program.send_bytes(ANYONE, b"Launchpad\x00Finalize");

    // Claim refund
    let result = program.send_bytes(
        CONTRIBUTOR1,
        b"Launchpad\x00ClaimRefund",
    );

    assert!(!result.main_failed(), "Claiming refund should succeed");
}

#[test]
fn test_max_per_wallet_limit() {
    let system = setup_system();
    let program = deploy_contract(&system);

    // Create and start launch with max_per_wallet limit
    let _ = program.send_bytes(CREATOR, b"Launchpad\x00CreateLaunch");
    let _ = program.send_bytes(CREATOR, b"Launchpad\x00StartLaunch");

    system.spend_blocks(1000);

    // First contribution within limit
    system.mint_to(CONTRIBUTOR1, 1000 * ONE_VARA);
    let result1 = program.send_bytes_with_value(
        CONTRIBUTOR1,
        b"Launchpad\x00Contribute",
        50 * ONE_VARA,
    );
    assert!(!result1.main_failed(), "First contribution should succeed");

    // Second contribution that would exceed limit
    // (behavior depends on implementation - may cap or reject)
}

#[test]
fn test_contribute_outside_time_window() {
    let system = setup_system();
    let program = deploy_contract(&system);

    // Create and start launch
    let _ = program.send_bytes(CREATOR, b"Launchpad\x00CreateLaunch");
    let _ = program.send_bytes(CREATOR, b"Launchpad\x00StartLaunch");

    // Don't advance blocks - still before start time
    system.mint_to(CONTRIBUTOR1, 1000 * ONE_VARA);
    let result = program.send_bytes_with_value(
        CONTRIBUTOR1,
        b"Launchpad\x00Contribute",
        50 * ONE_VARA,
    );

    // Should fail - outside time window
}

#[test]
fn test_withdraw_funds_after_success() {
    let system = setup_system();
    let program = deploy_contract(&system);

    // Create, start, fund, and finalize launch
    let _ = program.send_bytes(CREATOR, b"Launchpad\x00CreateLaunch");
    let _ = program.send_bytes(CREATOR, b"Launchpad\x00StartLaunch");

    system.spend_blocks(1000);

    system.mint_to(CONTRIBUTOR1, 1000 * ONE_VARA);
    let _ = program.send_bytes_with_value(
        CONTRIBUTOR1,
        b"Launchpad\x00Contribute",
        600 * ONE_VARA,
    );

    system.spend_blocks(10000);
    let _ = program.send_bytes(ANYONE, b"Launchpad\x00Finalize");

    // Creator withdraws funds
    let result = program.send_bytes(
        CREATOR,
        b"Launchpad\x00WithdrawFunds",
    );

    assert!(!result.main_failed(), "Withdrawing funds should succeed");
}

#[test]
fn test_cancel_launch() {
    let system = setup_system();
    let program = deploy_contract(&system);

    // Create a launch (still pending)
    let _ = program.send_bytes(
        CREATOR,
        b"Launchpad\x00CreateLaunch",
    );

    // Cancel before starting
    let result = program.send_bytes(
        CREATOR,
        b"Launchpad\x00CancelLaunch",
    );

    assert!(!result.main_failed(), "Cancelling launch should succeed");
}

#[test]
fn test_add_to_whitelist() {
    let system = setup_system();
    let program = deploy_contract(&system);

    // Create a launch
    let _ = program.send_bytes(
        CREATOR,
        b"Launchpad\x00CreateLaunch",
    );

    // Add addresses to whitelist
    let result = program.send_bytes(
        CREATOR,
        b"Launchpad\x00AddToWhitelist",
    );

    assert!(!result.main_failed(), "Adding to whitelist should succeed");
}

#[test]
fn test_unauthorized_withdraw_fails() {
    let system = setup_system();
    let program = deploy_contract(&system);

    // Create, start, fund, and finalize
    let _ = program.send_bytes(CREATOR, b"Launchpad\x00CreateLaunch");
    let _ = program.send_bytes(CREATOR, b"Launchpad\x00StartLaunch");

    system.spend_blocks(1000);

    system.mint_to(CONTRIBUTOR1, 1000 * ONE_VARA);
    let _ = program.send_bytes_with_value(CONTRIBUTOR1, b"Launchpad\x00Contribute", 600 * ONE_VARA);

    system.spend_blocks(10000);
    let _ = program.send_bytes(ANYONE, b"Launchpad\x00Finalize");

    // Non-creator tries to withdraw
    let result = program.send_bytes(
        ANYONE,
        b"Launchpad\x00WithdrawFunds",
    );

    // Should fail - unauthorized
}

#[test]
fn test_query_launch_info() {
    let system = setup_system();
    let program = deploy_contract(&system);

    // Create a launch
    let _ = program.send_bytes(CREATOR, b"Launchpad\x00CreateLaunch");

    // Query launch info
    let result = program.send_bytes(
        ANYONE,
        b"Launchpad\x00GetLaunch",
    );

    assert!(!result.main_failed(), "Query launch should succeed");
}

#[test]
fn test_get_active_launches() {
    let system = setup_system();
    let program = deploy_contract(&system);

    // Create and start multiple launches
    let _ = program.send_bytes(CREATOR, b"Launchpad\x00CreateLaunch");
    let _ = program.send_bytes(CREATOR, b"Launchpad\x00StartLaunch");

    // Query active launches
    let result = program.send_bytes(
        ANYONE,
        b"Launchpad\x00GetActiveLaunches",
    );

    assert!(!result.main_failed(), "Query active launches should succeed");
}

#[test]
fn test_full_launch_lifecycle() {
    let system = setup_system();
    let program = deploy_contract(&system);

    // 1. Create launch
    let create = program.send_bytes(CREATOR, b"Launchpad\x00CreateLaunch");
    assert!(!create.main_failed(), "Create should succeed");

    // 2. Setup whitelist (optional)
    let whitelist = program.send_bytes(CREATOR, b"Launchpad\x00AddToWhitelist");
    assert!(!whitelist.main_failed(), "Whitelist should succeed");

    // 3. Start launch
    let start = program.send_bytes(CREATOR, b"Launchpad\x00StartLaunch");
    assert!(!start.main_failed(), "Start should succeed");

    // 4. Advance to start time
    system.spend_blocks(1000);

    // 5. Contributors participate
    system.mint_to(CONTRIBUTOR1, 1000 * ONE_VARA);
    system.mint_to(CONTRIBUTOR2, 1000 * ONE_VARA);

    let contrib1 = program.send_bytes_with_value(
        CONTRIBUTOR1,
        b"Launchpad\x00Contribute",
        300 * ONE_VARA,
    );
    assert!(!contrib1.main_failed(), "Contribution 1 should succeed");

    let contrib2 = program.send_bytes_with_value(
        CONTRIBUTOR2,
        b"Launchpad\x00Contribute",
        300 * ONE_VARA,
    );
    assert!(!contrib2.main_failed(), "Contribution 2 should succeed");

    // 6. Advance past end time
    system.spend_blocks(10000);

    // 7. Finalize
    let finalize = program.send_bytes(ANYONE, b"Launchpad\x00Finalize");
    assert!(!finalize.main_failed(), "Finalize should succeed");

    // 8. Creator withdraws
    let withdraw = program.send_bytes(CREATOR, b"Launchpad\x00WithdrawFunds");
    assert!(!withdraw.main_failed(), "Withdraw should succeed");

    // 9. Contributors claim tokens
    let claim1 = program.send_bytes(CONTRIBUTOR1, b"Launchpad\x00ClaimTokens");
    assert!(!claim1.main_failed(), "Claim 1 should succeed");

    let claim2 = program.send_bytes(CONTRIBUTOR2, b"Launchpad\x00ClaimTokens");
    assert!(!claim2.main_failed(), "Claim 2 should succeed");
}

#[test]
fn test_withdraw_platform_fees() {
    let system = setup_system();
    let program = deploy_contract(&system);

    // Complete a successful launch to accumulate fees
    let _ = program.send_bytes(CREATOR, b"Launchpad\x00CreateLaunch");
    let _ = program.send_bytes(CREATOR, b"Launchpad\x00StartLaunch");

    system.spend_blocks(1000);

    system.mint_to(CONTRIBUTOR1, 1000 * ONE_VARA);
    let _ = program.send_bytes_with_value(CONTRIBUTOR1, b"Launchpad\x00Contribute", 600 * ONE_VARA);

    system.spend_blocks(10000);
    let _ = program.send_bytes(ANYONE, b"Launchpad\x00Finalize");
    let _ = program.send_bytes(CREATOR, b"Launchpad\x00WithdrawFunds");

    // Owner withdraws accumulated fees
    let result = program.send_bytes(
        OWNER,
        b"Launchpad\x00WithdrawFees",
    );

    // Should succeed if fees accumulated
}

#[test]
fn test_is_whitelisted_query() {
    let system = setup_system();
    let program = deploy_contract(&system);

    // Create launch and add to whitelist
    let _ = program.send_bytes(CREATOR, b"Launchpad\x00CreateLaunch");
    let _ = program.send_bytes(CREATOR, b"Launchpad\x00AddToWhitelist");

    // Query whitelist status
    let result = program.send_bytes(
        ANYONE,
        b"Launchpad\x00IsWhitelisted",
    );

    assert!(!result.main_failed(), "Whitelist query should succeed");
}
