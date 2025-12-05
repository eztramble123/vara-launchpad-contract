# Vara Launchpad Contract

A production-ready token launchpad smart contract for the [Vara Network](https://vara.network), built with [Sails-RS](https://github.com/gear-tech/sails).

## Overview

This contract enables project creators to conduct fair token sales on Vara Network with:

- **Clean State Machine**: Well-defined lifecycle states with proper transitions
- **Whitelist Support**: Optional address-based access control
- **Contribution Limits**: Per-wallet caps with automatic excess refunds
- **Soft/Hard Caps**: Minimum raise threshold with refunds if not met
- **Vesting Support**: Linear vesting with cliff periods for token distribution
- **Platform Fees**: Configurable fee on successful launches (default 2%)
- **Pause/Resume**: Emergency controls for contract owner

## Quick Start

### Prerequisites

- Rust 1.88.0+ (`rustup update`)
- WASM target: `rustup target add wasm32v1-none`

### Build

```bash
cargo build --release -p launchpad
```

Output: `target/wasm32-gear/release/launchpad.opt.wasm`

### Test

```bash
cargo test --release -p launchpad
```

All 17 integration tests should pass.

## State Machine

```
Pending ────────────► Active
   │                    │
   │                    ▼
   │              ┌─────────┐
   │              │  Ended  │
   │              └────┬────┘
   │                   │
   │         ┌─────────┴─────────┐
   │         ▼                   ▼
   │    Succeeded            Failed
   │         │                   │
   │         ▼                   ▼
   │  DistributionPending  RefundAvailable
   │         │                   │
   └─────────┴─────────┬─────────┘
                       ▼
                  Finalized
```

### State Descriptions

| State | Description |
|-------|-------------|
| `Pending` | Launch created, not yet started. Configure whitelist here. |
| `Active` | Accepting contributions within time window. |
| `Ended` | Time expired or fully subscribed, outcome pending. |
| `Succeeded` | Soft cap met, tokens ready for distribution. |
| `DistributionPending` | Distribution in progress, claims enabled. |
| `Failed` | Soft cap not met, refunds available. |
| `Cancelled` | Creator cancelled the launch. |
| `RefundAvailable` | Failed/cancelled, users can claim refunds. |
| `Finalized` | All operations complete. |

## API Reference

### Commands (State-Changing)

| Method | Parameters | Description |
|--------|------------|-------------|
| `create_launch` | `CreateLaunchInput` | Create new token launch |
| `add_to_whitelist` | `launch_id, addresses[]` | Add addresses to whitelist |
| `start_launch` | `launch_id` | Activate launch (creator only) |
| `mark_tokens_deposited` | `launch_id` | Mark tokens as deposited |
| `contribute` | `launch_id` + VARA value | Contribute to launch |
| `finalize` | `launch_id` | Finalize after end time |
| `claim_tokens` | `launch_id` | Claim purchased tokens |
| `claim_refund` | `launch_id` | Claim refund (failed/cancelled) |
| `withdraw_funds` | `launch_id` | Withdraw raised funds (creator) |
| `cancel_launch` | `launch_id` | Cancel launch |
| `withdraw_fees` | - | Withdraw platform fees (owner) |
| `pause` | - | Pause contract (owner) |
| `resume` | - | Resume contract (owner) |

### Queries (Read-Only)

| Method | Parameters | Returns |
|--------|------------|---------|
| `get_launch` | `launch_id` | `Option<Launch>` |
| `get_active_launches` | - | `Vec<Launch>` |
| `get_creator_launches` | `creator` | `Vec<Launch>` |
| `get_contribution` | `launch_id, user` | `Amount` |
| `get_tokens_purchased` | `launch_id, user` | `Amount` |
| `get_claimed` | `launch_id, user` | `Amount` |
| `get_claimable_tokens` | `launch_id, user` | `Amount` |
| `is_whitelisted` | `launch_id, address` | `bool` |
| `get_contributors` | `launch_id` | `Vec<ActorId>` |
| `get_launch_count` | - | `u64` |
| `get_accumulated_fees` | - | `Amount` |
| `get_available_fees` | - | `Amount` |
| `get_owner` | - | `ActorId` |
| `is_paused` | - | `bool` |

### CreateLaunchInput

```rust
pub struct CreateLaunchInput {
    pub title: String,
    pub description: String,
    pub token_address: ActorId,
    pub total_tokens: Amount,
    pub price_per_token: Amount,
    pub min_raise: Amount,        // Soft cap
    pub max_raise: Amount,        // Hard cap
    pub max_per_wallet: Amount,
    pub start_time: BlockNumber,
    pub end_time: BlockNumber,
    pub whitelist_enabled: bool,
    pub vesting_config: Option<VestingConfig>,
}
```

### Events

| Event | Description |
|-------|-------------|
| `LaunchCreated` | New launch created with full parameters |
| `LaunchStarted` | Launch activated |
| `SaleEnded` | Sale period ended (time/fully subscribed) |
| `SaleFullySubscribed` | Hard cap reached |
| `LaunchSucceeded` | Soft cap met |
| `LaunchFailed` | Soft cap not met |
| `LaunchCancelled` | Launch cancelled |
| `DistributionPending` | Distribution phase started |
| `RefundsAvailable` | Refunds enabled |
| `Contributed` | User contributed |
| `TokensClaimed` | Tokens claimed |
| `TokenTransferFailed` | Token transfer failed (for retry) |
| `RefundClaimed` | Refund claimed |
| `FundsWithdrawn` | Creator withdrew funds |
| `FeesWithdrawn` | Platform fees withdrawn |
| `WhitelistUpdated` | Whitelist modified |
| `TokensDeposited` | Creator marked tokens deposited |
| `LaunchFinalized` | All operations complete |
| `Paused` | Contract paused |
| `Resumed` | Contract resumed |

## Usage Examples

### Creating a Launch

```rust
let input = CreateLaunchInput {
    title: "My Token Sale".into(),
    description: "Fair launch of MyToken".into(),
    token_address: my_token_contract,
    total_tokens: 1_000_000 * ONE_TOKEN,
    price_per_token: ONE_VARA / 100,  // 0.01 VARA per token
    min_raise: 500 * ONE_VARA,         // Soft cap
    max_raise: 10_000 * ONE_VARA,      // Hard cap
    max_per_wallet: 100 * ONE_VARA,
    start_time: current_block + 1000,
    end_time: current_block + 10000,
    whitelist_enabled: true,
    vesting_config: Some(VestingConfig {
        start_block: current_block + 10000,
        cliff_duration: 5000,
        vesting_duration: 50000,
    }),
};

let launch_id = launchpad.create_launch(input)?;
```

### Full Lifecycle

```rust
// 1. Create launch
let launch_id = launchpad.create_launch(input)?;

// 2. Setup whitelist (optional)
launchpad.add_to_whitelist(launch_id, vec![user1, user2])?;

// 3. Start launch
launchpad.start_launch(launch_id)?;

// 4. Users contribute (during time window)
launchpad.contribute(launch_id)?; // with VARA value

// 5. Finalize after end time
launchpad.finalize(launch_id)?;

// 6a. If successful:
launchpad.withdraw_funds(launch_id)?;  // Creator gets funds
launchpad.claim_tokens(launch_id)?;    // Users get tokens

// 6b. If failed:
launchpad.claim_refund(launch_id)?;    // Users get refunds
```

## Project Structure

```
vara-launchpad-contract/
├── contracts/launchpad/
│   ├── app/src/lib.rs      # Core contract logic (~1200 lines)
│   ├── src/lib.rs          # WASM entry point
│   ├── client/             # Client library
│   ├── tests/gtest.rs      # Integration tests (17 tests)
│   └── build.rs            # Build script
├── shared/
│   ├── src/
│   │   ├── types.rs        # Common types (Id, Amount, VestingConfig)
│   │   └── errors.rs       # Error definitions (ContractError)
│   └── Cargo.toml
├── docs/
│   ├── deployment-guide.md
│   ├── security-checklist.md
│   └── integration-patterns.md
├── Cargo.toml              # Workspace configuration
└── rust-toolchain.toml
```

## Technical Stack

| Component | Version |
|-----------|---------|
| Sails-RS | 0.9.1 |
| Gstd/Gtest | 1.9.1 |
| Rust Edition | 2021 |
| Build Target | wasm32v1-none |

## Safety Features

- **Checked Arithmetic**: All calculations use `saturating_*` or `checked_*` operations
- **CEI Pattern**: State updated before external calls
- **Authorization**: Caller validation for all privileged operations
- **Input Validation**: Comprehensive parameter validation
- **Automatic Refunds**: Excess contributions refunded immediately
- **Double-Claim Prevention**: Claimed amounts tracked per user
- **Pause Mechanism**: Emergency pause by platform owner

## Deployment

1. Build the contract:
   ```bash
   cargo build --release -p launchpad
   ```

2. Deploy to Vara Network using:
   - [Gear IDEA](https://idea.gear-tech.io/)
   - `gcli` command line tool
   - Programmatic deployment via `gclient`

3. Initialize with constructor:
   - `New` - Default 2% platform fee
   - `NewWithFee(fee_basis_points)` - Custom fee (100 = 1%)

## Documentation

- [Deployment Guide](./docs/deployment-guide.md) - Deployment instructions
- [Security Checklist](./docs/security-checklist.md) - Security considerations
- [Integration Patterns](./docs/integration-patterns.md) - Integration examples
- [Contract README](./contracts/launchpad/README.md) - Detailed contract docs

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines.

## License

Licensed under MIT OR Apache-2.0.
