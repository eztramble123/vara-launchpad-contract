//! Integration tests for Launchpad v2 contract.

use gtest::{Program, System};
use launchpad_app::{CreateLaunchInput, CONTRACT_NAME, CONTRACT_VERSION};
use sails_rs::prelude::ActorId;
use sails_rs::Encode;

// User IDs must be >= 100 to be valid in gtest
const OWNER: u64 = 100;
const CREATOR: u64 = 101;
const CONTRIBUTOR1: u64 = 102;
const CONTRIBUTOR2: u64 = 103;
const NON_WHITELISTED: u64 = 104;
const ANYONE: u64 = 105;

const ONE_VARA: u128 = 1_000_000_000_000; // 10^12
const EXISTENTIAL_DEPOSIT: u128 = 10 * ONE_VARA;

// Token address (dummy for testing)
const TOKEN_ADDRESS: u64 = 200;

/// Encode a Sails constructor name
fn encode_constructor(name: &str) -> Vec<u8> {
    name.encode()
}

/// Encode a Sails service call with parameters
fn encode_call<T: Encode>(service: &str, method: &str, params: T) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend(service.encode());
    payload.extend(method.encode());
    payload.extend(params.encode());
    payload
}

/// Encode a Sails service call without parameters
fn encode_call_no_params(service: &str, method: &str) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend(service.encode());
    payload.extend(method.encode());
    payload
}

fn setup_system() -> System {
    let system = System::new();
    system.init_logger();

    // Mint balances for all test users
    system.mint_to(OWNER, EXISTENTIAL_DEPOSIT * 1000);
    system.mint_to(CREATOR, EXISTENTIAL_DEPOSIT * 1000);
    system.mint_to(CONTRIBUTOR1, EXISTENTIAL_DEPOSIT * 1000);
    system.mint_to(CONTRIBUTOR2, EXISTENTIAL_DEPOSIT * 1000);
    system.mint_to(NON_WHITELISTED, EXISTENTIAL_DEPOSIT * 1000);
    system.mint_to(ANYONE, EXISTENTIAL_DEPOSIT * 1000);

    system
}

fn deploy_contract(system: &System) -> Program<'_> {
    let program = Program::from_file(
        system,
        "../../target/wasm32-gear/release/launchpad.opt.wasm",
    );

    // Initialize with OWNER as the deployer (default 2% fee)
    let init_payload = encode_constructor("New");
    let init_msg_id = program.send_bytes(OWNER, init_payload);
    let result = system.run_next_block();

    if !result.succeed.contains(&init_msg_id) {
        panic!("Contract init failed. Check WASM file exists and is valid.");
    }

    // Note: In production, owner would need to call set_vft_code_id()
    // before creating launches. Tests mock this behavior.

    program
}

/// Create a standard launch input for testing
fn create_test_launch_input(system: &System) -> CreateLaunchInput {
    let current_block = system.block_height();

    CreateLaunchInput {
        // Token creation parameters
        token_name: "Test Token".into(),
        token_symbol: "TEST".into(),
        
        // Launch parameters
        title: "Test Token Launch".into(),
        description: "A test token launch for integration testing".into(),
        total_tokens: 1_000_000 * ONE_VARA,
        price_per_token: ONE_VARA / 1000, // 0.001 VARA per token
        min_raise: 100 * ONE_VARA,
        max_raise: 1000 * ONE_VARA,
        max_per_wallet: 200 * ONE_VARA,
        start_time: current_block + 10,
        end_time: current_block + 10000,
        whitelist_enabled: false,
        vesting_config: None,
    }
}

/// Advance blocks to simulate time passing
fn advance_blocks(system: &System, count: u32) {
    for _ in 0..count {
        system.run_next_block();
    }
}

// =============================================================================
// BASIC TESTS
// =============================================================================

#[test]
fn test_contract_initialization() {
    let system = setup_system();
    let _program = deploy_contract(&system);
    // If we get here, initialization succeeded
}

#[test]
fn test_create_launch() {
    let system = setup_system();
    let program = deploy_contract(&system);

    let input = create_test_launch_input(&system);
    let payload = encode_call("Launchpad", "CreateLaunch", input);

    let msg_id = program.send_bytes(CREATOR, payload);
    let result = system.run_next_block();

    assert!(
        result.succeed.contains(&msg_id),
        "CreateLaunch should succeed"
    );
}

#[test]
fn test_start_launch() {
    let system = setup_system();
    let program = deploy_contract(&system);

    // Create launch
    let input = create_test_launch_input(&system);
    let msg_id = program.send_bytes(CREATOR, encode_call("Launchpad", "CreateLaunch", input));
    let result = system.run_next_block();
    assert!(result.succeed.contains(&msg_id));

    // Start launch (launch_id = 0)
    let launch_id: u64 = 0;
    let msg_id = program.send_bytes(CREATOR, encode_call("Launchpad", "StartLaunch", launch_id));
    let result = system.run_next_block();

    assert!(
        result.succeed.contains(&msg_id),
        "StartLaunch should succeed"
    );
}

#[test]
fn test_contribute() {
    let system = setup_system();
    let program = deploy_contract(&system);

    // Create and start launch
    let input = create_test_launch_input(&system);
    program.send_bytes(CREATOR, encode_call("Launchpad", "CreateLaunch", input));
    system.run_next_block();

    let launch_id: u64 = 0;
    program.send_bytes(CREATOR, encode_call("Launchpad", "StartLaunch", launch_id));
    system.run_next_block();

    // Advance to start time
    advance_blocks(&system, 15);

    // Contribute
    let msg_id = program.send_bytes_with_value(
        CONTRIBUTOR1,
        encode_call("Launchpad", "Contribute", launch_id),
        50 * ONE_VARA,
    );
    let result = system.run_next_block();

    assert!(
        result.succeed.contains(&msg_id),
        "Contribute should succeed"
    );
}

#[test]
fn test_whitelist_functionality() {
    let system = setup_system();
    let program = deploy_contract(&system);

    // Create launch with whitelist enabled
    let mut input = create_test_launch_input(&system);
    input.whitelist_enabled = true;

    program.send_bytes(CREATOR, encode_call("Launchpad", "CreateLaunch", input));
    system.run_next_block();

    let launch_id: u64 = 0;

    // Add CONTRIBUTOR1 to whitelist
    let addresses: Vec<ActorId> = vec![ActorId::from(CONTRIBUTOR1)];
    let msg_id = program.send_bytes(
        CREATOR,
        encode_call("Launchpad", "AddToWhitelist", (launch_id, addresses)),
    );
    let result = system.run_next_block();

    assert!(
        result.succeed.contains(&msg_id),
        "AddToWhitelist should succeed"
    );

    // Start launch
    program.send_bytes(CREATOR, encode_call("Launchpad", "StartLaunch", launch_id));
    system.run_next_block();

    // Advance to start time
    advance_blocks(&system, 15);

    // Whitelisted user can contribute
    let msg_id = program.send_bytes_with_value(
        CONTRIBUTOR1,
        encode_call("Launchpad", "Contribute", launch_id),
        50 * ONE_VARA,
    );
    let result = system.run_next_block();

    assert!(
        result.succeed.contains(&msg_id),
        "Whitelisted contributor should succeed"
    );

    // Non-whitelisted user should fail
    let msg_id = program.send_bytes_with_value(
        NON_WHITELISTED,
        encode_call("Launchpad", "Contribute", launch_id),
        50 * ONE_VARA,
    );
    let result = system.run_next_block();

    assert!(
        result.failed.contains(&msg_id),
        "Non-whitelisted contributor should fail"
    );
}

#[test]
fn test_finalize_successful_launch() {
    let system = setup_system();
    let program = deploy_contract(&system);

    // Create launch with low min_raise
    let mut input = create_test_launch_input(&system);
    input.min_raise = 50 * ONE_VARA;

    program.send_bytes(CREATOR, encode_call("Launchpad", "CreateLaunch", input));
    system.run_next_block();

    let launch_id: u64 = 0;
    program.send_bytes(CREATOR, encode_call("Launchpad", "StartLaunch", launch_id));
    system.run_next_block();

    // Advance to start time
    advance_blocks(&system, 15);

    // Contribute enough to meet min_raise
    program.send_bytes_with_value(
        CONTRIBUTOR1,
        encode_call("Launchpad", "Contribute", launch_id),
        100 * ONE_VARA,
    );
    system.run_next_block();

    // Advance past end time
    advance_blocks(&system, 10000);

    // Finalize
    let msg_id = program.send_bytes(ANYONE, encode_call("Launchpad", "Finalize", launch_id));
    let result = system.run_next_block();

    assert!(
        result.succeed.contains(&msg_id),
        "Finalize should succeed for successful launch"
    );
}

#[test]
fn test_finalize_failed_launch() {
    let system = setup_system();
    let program = deploy_contract(&system);

    // Create launch with high min_raise
    let mut input = create_test_launch_input(&system);
    input.min_raise = 500 * ONE_VARA;

    program.send_bytes(CREATOR, encode_call("Launchpad", "CreateLaunch", input));
    system.run_next_block();

    let launch_id: u64 = 0;
    program.send_bytes(CREATOR, encode_call("Launchpad", "StartLaunch", launch_id));
    system.run_next_block();

    // Advance to start time
    advance_blocks(&system, 15);

    // Contribute less than min_raise
    program.send_bytes_with_value(
        CONTRIBUTOR1,
        encode_call("Launchpad", "Contribute", launch_id),
        50 * ONE_VARA,
    );
    system.run_next_block();

    // Advance past end time
    advance_blocks(&system, 10000);

    // Finalize - should mark as failed
    let msg_id = program.send_bytes(ANYONE, encode_call("Launchpad", "Finalize", launch_id));
    let result = system.run_next_block();

    assert!(
        result.succeed.contains(&msg_id),
        "Finalize should succeed (marking launch as failed)"
    );
}

#[test]
fn test_claim_refund() {
    let system = setup_system();
    let program = deploy_contract(&system);

    // Create launch with high min_raise (will fail)
    let mut input = create_test_launch_input(&system);
    input.min_raise = 500 * ONE_VARA;

    program.send_bytes(CREATOR, encode_call("Launchpad", "CreateLaunch", input));
    system.run_next_block();

    let launch_id: u64 = 0;
    program.send_bytes(CREATOR, encode_call("Launchpad", "StartLaunch", launch_id));
    system.run_next_block();

    advance_blocks(&system, 15);

    // Contribute
    program.send_bytes_with_value(
        CONTRIBUTOR1,
        encode_call("Launchpad", "Contribute", launch_id),
        50 * ONE_VARA,
    );
    system.run_next_block();

    // Advance past end time and finalize
    advance_blocks(&system, 10000);
    program.send_bytes(ANYONE, encode_call("Launchpad", "Finalize", launch_id));
    system.run_next_block();

    // Claim refund
    let msg_id = program.send_bytes(
        CONTRIBUTOR1,
        encode_call("Launchpad", "ClaimRefund", launch_id),
    );
    let result = system.run_next_block();

    assert!(
        result.succeed.contains(&msg_id),
        "ClaimRefund should succeed for failed launch"
    );
}

#[test]
fn test_withdraw_funds() {
    let system = setup_system();
    let program = deploy_contract(&system);

    // Create launch with low min_raise
    let mut input = create_test_launch_input(&system);
    input.min_raise = 50 * ONE_VARA;

    program.send_bytes(CREATOR, encode_call("Launchpad", "CreateLaunch", input));
    system.run_next_block();

    let launch_id: u64 = 0;
    program.send_bytes(CREATOR, encode_call("Launchpad", "StartLaunch", launch_id));
    system.run_next_block();

    advance_blocks(&system, 15);

    // Contribute enough
    program.send_bytes_with_value(
        CONTRIBUTOR1,
        encode_call("Launchpad", "Contribute", launch_id),
        100 * ONE_VARA,
    );
    system.run_next_block();

    // Advance and finalize
    advance_blocks(&system, 10000);
    program.send_bytes(ANYONE, encode_call("Launchpad", "Finalize", launch_id));
    system.run_next_block();

    // Creator withdraws funds
    let msg_id = program.send_bytes(
        CREATOR,
        encode_call("Launchpad", "WithdrawFunds", launch_id),
    );
    let result = system.run_next_block();

    assert!(
        result.succeed.contains(&msg_id),
        "WithdrawFunds should succeed for creator"
    );
}

#[test]
fn test_unauthorized_withdraw_fails() {
    let system = setup_system();
    let program = deploy_contract(&system);

    let mut input = create_test_launch_input(&system);
    input.min_raise = 50 * ONE_VARA;

    program.send_bytes(CREATOR, encode_call("Launchpad", "CreateLaunch", input));
    system.run_next_block();

    let launch_id: u64 = 0;
    program.send_bytes(CREATOR, encode_call("Launchpad", "StartLaunch", launch_id));
    system.run_next_block();

    advance_blocks(&system, 15);

    program.send_bytes_with_value(
        CONTRIBUTOR1,
        encode_call("Launchpad", "Contribute", launch_id),
        100 * ONE_VARA,
    );
    system.run_next_block();

    advance_blocks(&system, 10000);
    program.send_bytes(ANYONE, encode_call("Launchpad", "Finalize", launch_id));
    system.run_next_block();

    // Non-creator tries to withdraw
    let msg_id = program.send_bytes(
        ANYONE,
        encode_call("Launchpad", "WithdrawFunds", launch_id),
    );
    let result = system.run_next_block();

    assert!(
        result.failed.contains(&msg_id),
        "Unauthorized withdraw should fail"
    );
}

#[test]
fn test_cancel_launch() {
    let system = setup_system();
    let program = deploy_contract(&system);

    let input = create_test_launch_input(&system);
    program.send_bytes(CREATOR, encode_call("Launchpad", "CreateLaunch", input));
    system.run_next_block();

    let launch_id: u64 = 0;

    // Cancel before starting
    let msg_id = program.send_bytes(
        CREATOR,
        encode_call("Launchpad", "CancelLaunch", launch_id),
    );
    let result = system.run_next_block();

    assert!(
        result.succeed.contains(&msg_id),
        "CancelLaunch should succeed for pending launch"
    );
}

#[test]
fn test_pause_resume() {
    let system = setup_system();
    let program = deploy_contract(&system);

    // Pause (owner only)
    let msg_id = program.send_bytes(OWNER, encode_call_no_params("Launchpad", "Pause"));
    let result = system.run_next_block();
    assert!(result.succeed.contains(&msg_id), "Pause should succeed");

    // Try to create launch while paused - should fail
    let input = create_test_launch_input(&system);
    let msg_id = program.send_bytes(CREATOR, encode_call("Launchpad", "CreateLaunch", input.clone()));
    let result = system.run_next_block();
    assert!(result.failed.contains(&msg_id), "CreateLaunch should fail when paused");

    // Resume
    let msg_id = program.send_bytes(OWNER, encode_call_no_params("Launchpad", "Resume"));
    let result = system.run_next_block();
    assert!(result.succeed.contains(&msg_id), "Resume should succeed");

    // Now create launch should work
    let msg_id = program.send_bytes(CREATOR, encode_call("Launchpad", "CreateLaunch", input));
    let result = system.run_next_block();
    assert!(result.succeed.contains(&msg_id), "CreateLaunch should succeed after resume");
}

#[test]
fn test_query_launch() {
    let system = setup_system();
    let program = deploy_contract(&system);

    let input = create_test_launch_input(&system);
    program.send_bytes(CREATOR, encode_call("Launchpad", "CreateLaunch", input));
    system.run_next_block();

    let launch_id: u64 = 0;

    // Query launch
    let msg_id = program.send_bytes(
        ANYONE,
        encode_call("Launchpad", "GetLaunch", launch_id),
    );
    let result = system.run_next_block();

    assert!(
        result.succeed.contains(&msg_id),
        "GetLaunch query should succeed"
    );
}

#[test]
fn test_query_active_launches() {
    let system = setup_system();
    let program = deploy_contract(&system);

    // Create and start a launch
    let input = create_test_launch_input(&system);
    program.send_bytes(CREATOR, encode_call("Launchpad", "CreateLaunch", input));
    system.run_next_block();

    let launch_id: u64 = 0;
    program.send_bytes(CREATOR, encode_call("Launchpad", "StartLaunch", launch_id));
    system.run_next_block();

    // Query active launches
    let msg_id = program.send_bytes(
        ANYONE,
        encode_call_no_params("Launchpad", "GetActiveLaunches"),
    );
    let result = system.run_next_block();

    assert!(
        result.succeed.contains(&msg_id),
        "GetActiveLaunches query should succeed"
    );
}

#[test]
fn test_full_launch_lifecycle() {
    let system = setup_system();
    let program = deploy_contract(&system);

    // 1. Create launch
    let mut input = create_test_launch_input(&system);
    input.min_raise = 100 * ONE_VARA;

    let msg_id = program.send_bytes(CREATOR, encode_call("Launchpad", "CreateLaunch", input));
    let result = system.run_next_block();
    assert!(result.succeed.contains(&msg_id), "Create should succeed");

    let launch_id: u64 = 0;

    // 2. Start launch
    let msg_id = program.send_bytes(CREATOR, encode_call("Launchpad", "StartLaunch", launch_id));
    let result = system.run_next_block();
    assert!(result.succeed.contains(&msg_id), "Start should succeed");

    // 3. Advance to start time
    advance_blocks(&system, 15);

    // 4. Contributors participate
    let msg_id = program.send_bytes_with_value(
        CONTRIBUTOR1,
        encode_call("Launchpad", "Contribute", launch_id),
        60 * ONE_VARA,
    );
    let result = system.run_next_block();
    assert!(result.succeed.contains(&msg_id), "Contribution 1 should succeed");

    let msg_id = program.send_bytes_with_value(
        CONTRIBUTOR2,
        encode_call("Launchpad", "Contribute", launch_id),
        60 * ONE_VARA,
    );
    let result = system.run_next_block();
    assert!(result.succeed.contains(&msg_id), "Contribution 2 should succeed");

    // 5. Advance past end time
    advance_blocks(&system, 10000);

    // 6. Finalize
    let msg_id = program.send_bytes(ANYONE, encode_call("Launchpad", "Finalize", launch_id));
    let result = system.run_next_block();
    assert!(result.succeed.contains(&msg_id), "Finalize should succeed");

    // 7. Creator withdraws funds
    let msg_id = program.send_bytes(CREATOR, encode_call("Launchpad", "WithdrawFunds", launch_id));
    let result = system.run_next_block();
    assert!(result.succeed.contains(&msg_id), "Withdraw should succeed");

    // 8. Contributors claim tokens
    let msg_id = program.send_bytes(CONTRIBUTOR1, encode_call("Launchpad", "ClaimTokens", launch_id));
    let result = system.run_next_block();
    assert!(result.succeed.contains(&msg_id), "Claim 1 should succeed");

    let msg_id = program.send_bytes(CONTRIBUTOR2, encode_call("Launchpad", "ClaimTokens", launch_id));
    let result = system.run_next_block();
    assert!(result.succeed.contains(&msg_id), "Claim 2 should succeed");
}

#[test]
fn test_contribution_limits() {
    let system = setup_system();
    let program = deploy_contract(&system);

    // Create launch with low max_per_wallet
    let mut input = create_test_launch_input(&system);
    input.max_per_wallet = 50 * ONE_VARA;

    program.send_bytes(CREATOR, encode_call("Launchpad", "CreateLaunch", input));
    system.run_next_block();

    let launch_id: u64 = 0;
    program.send_bytes(CREATOR, encode_call("Launchpad", "StartLaunch", launch_id));
    system.run_next_block();

    advance_blocks(&system, 15);

    // First contribution at limit
    let msg_id = program.send_bytes_with_value(
        CONTRIBUTOR1,
        encode_call("Launchpad", "Contribute", launch_id),
        50 * ONE_VARA,
    );
    let result = system.run_next_block();
    assert!(result.succeed.contains(&msg_id), "First contribution should succeed");

    // Second contribution should fail (at limit)
    let msg_id = program.send_bytes_with_value(
        CONTRIBUTOR1,
        encode_call("Launchpad", "Contribute", launch_id),
        10 * ONE_VARA,
    );
    let result = system.run_next_block();
    assert!(result.failed.contains(&msg_id), "Over-limit contribution should fail");
}

#[test]
fn test_withdraw_platform_fees() {
    let system = setup_system();
    let program = deploy_contract(&system);

    // Complete a successful launch
    let mut input = create_test_launch_input(&system);
    input.min_raise = 50 * ONE_VARA;

    program.send_bytes(CREATOR, encode_call("Launchpad", "CreateLaunch", input));
    system.run_next_block();

    let launch_id: u64 = 0;
    program.send_bytes(CREATOR, encode_call("Launchpad", "StartLaunch", launch_id));
    system.run_next_block();

    advance_blocks(&system, 15);

    program.send_bytes_with_value(
        CONTRIBUTOR1,
        encode_call("Launchpad", "Contribute", launch_id),
        100 * ONE_VARA,
    );
    system.run_next_block();

    advance_blocks(&system, 10000);
    program.send_bytes(ANYONE, encode_call("Launchpad", "Finalize", launch_id));
    system.run_next_block();

    // Creator withdraws (generates fees)
    program.send_bytes(CREATOR, encode_call("Launchpad", "WithdrawFunds", launch_id));
    system.run_next_block();

    // Owner withdraws fees
    let msg_id = program.send_bytes(OWNER, encode_call_no_params("Launchpad", "WithdrawFees"));
    let result = system.run_next_block();

    assert!(
        result.succeed.contains(&msg_id),
        "WithdrawFees should succeed for owner"
    );
}
