# Token Launchpad Contract

Fair token launch platform for the Vara Network with whitelists, contribution limits, and optional vesting.

## Overview

This contract implements a token launchpad where project creators can conduct fair token sales. It supports optional whitelists for exclusive access, maximum contribution limits per wallet, automatic refunds if minimum raise targets aren't met, and optional vesting schedules for purchased tokens. Ideal for IDOs, token generation events, and community-driven launches.

## Features

- **Token Sales**: Create launches with configurable price, caps, and timing
- **Whitelist Support**: Optional exclusive access for approved addresses
- **Contribution Limits**: Maximum per-wallet caps to ensure fair distribution
- **Soft/Hard Caps**: Minimum raise threshold with automatic refunds if not met
- **Optional Vesting**: Lock purchased tokens with cliff and linear release
- **Platform Fees**: Configurable fee on successful launches (default 2%)
- **Launch Management**: Start, cancel, and finalize launches

## State Structure

```rust
pub struct Launch {
    pub id: Id,
    pub creator: ActorId,
    pub title: String,
    pub description: String,
    pub token_address: ActorId,
    pub total_tokens: Amount,
    pub tokens_remaining: Amount,
    pub price_per_token: Amount,
    pub min_raise: Amount,           // Soft cap
    pub max_raise: Amount,           // Hard cap
    pub total_raised: Amount,
    pub max_per_wallet: Amount,
    pub start_time: BlockNumber,
    pub end_time: BlockNumber,
    pub whitelist: BTreeSet<ActorId>,
    pub whitelist_enabled: bool,
    pub contributions: BTreeMap<ActorId, Amount>,
    pub claimed: BTreeMap<ActorId, Amount>,
    pub vesting_config: Option<VestingConfig>,
    pub status: LaunchStatus,
    pub created_at: BlockNumber,
}

pub enum LaunchStatus {
    Pending,    // Setup phase
    Active,     // Accepting contributions
    Succeeded,  // Min raise met
    Failed,     // Min raise not met by deadline
    Cancelled,  // Creator cancelled
    Finalized,  // Funds withdrawn
}
```

## API

### Commands (State-Changing)

| Method | Parameters | Description |
|--------|------------|-------------|
| `create_launch` | `CreateLaunchInput` | Create a new token launch |
| `add_to_whitelist` | `launch_id: Id, addresses: Vec<ActorId>` | Add addresses to whitelist |
| `start_launch` | `launch_id: Id` | Activate the launch (creator only) |
| `contribute` | `launch_id: Id` | Contribute to launch (send VARA) |
| `finalize` | `launch_id: Id` | Finalize launch after end time |
| `claim_tokens` | `launch_id: Id` | Claim purchased tokens |
| `claim_refund` | `launch_id: Id` | Claim refund for failed launch |
| `withdraw_funds` | `launch_id: Id` | Withdraw raised funds (creator) |
| `cancel_launch` | `launch_id: Id` | Cancel the launch |
| `withdraw_fees` | - | Withdraw platform fees (owner) |

### CreateLaunchInput

```rust
pub struct CreateLaunchInput {
    // Token creation parameters
    pub token_name: String,        // Max 64 characters
    pub token_symbol: String,      // Max 10 characters

    // Launch parameters
    pub title: String,
    pub description: String,
    pub total_tokens: Amount,
    pub price_per_token: Amount,
    pub min_raise: Amount,
    pub max_raise: Amount,
    pub max_per_wallet: Amount,
    pub start_time: BlockNumber,
    pub end_time: BlockNumber,
    pub whitelist_enabled: bool,
    pub vesting_config: Option<VestingConfig>,
}
```

### Queries (Read-Only)

| Method | Parameters | Returns | Description |
|--------|------------|---------|-------------|
| `get_launch` | `launch_id: Id` | `Option<Launch>` | Get launch by ID |
| `get_creator_launches` | `creator: ActorId` | `Vec<Launch>` | Get launches by creator |
| `get_active_launches` | - | `Vec<Launch>` | Get all active launches |
| `get_contribution` | `launch_id: Id, contributor: ActorId` | `Amount` | Get contribution amount |
| `get_claimed` | `launch_id: Id, claimer: ActorId` | `Amount` | Get claimed tokens |
| `is_whitelisted` | `launch_id: Id, address: ActorId` | `bool` | Check whitelist status |
| `get_launch_count` | - | `u64` | Total launches created |
| `get_accumulated_fees` | - | `Amount` | Platform fees collected |

## Events

```rust
pub enum LaunchpadEvent {
    LaunchCreated { launch_id, creator, token_address, total_tokens, ... },
    LaunchStarted { launch_id },
    TokenDeployed { launch_id, token_address, name, symbol, total_supply },
    Contributed { launch_id, contributor, amount, tokens_purchased, refunded },
    TokensClaimed { launch_id, user, amount },
    RefundClaimed { launch_id, user, amount },
    FundsWithdrawn { launch_id, creator, amount, fee },
    LaunchSucceeded { launch_id, total_raised },
    LaunchFailed { launch_id, total_raised, min_raise },
    LaunchCancelled { launch_id, by },
    WhitelistUpdated { launch_id, addresses_added },
    SaleEnded { launch_id, total_raised, total_contributors, reason },
    SaleFullySubscribed { launch_id, total_raised },
    DistributionPending { launch_id },
    RefundsAvailable { launch_id, total_to_refund, num_contributors },
    TokenTransferFailed { launch_id, user, amount, reason },
    FeesWithdrawn { owner, amount, total_accumulated },
    TokensDeposited { launch_id, amount },
    LaunchFinalized { launch_id },
    Paused { by },
    Resumed { by },
    FeeRecipientUpdated { old, new },
    GasConfigUpdated { gas_for_program, gas_for_reply },
    AdminForceRefund { launch_id, user, amount },
    TokensRescued { token_address, amount, to },
}
```

## Usage Example

### Creating a Launch

```rust
let input = CreateLaunchInput {
    title: "My Token Sale".into(),
    description: "Fair launch of MyToken".into(),
    token_address: my_token_contract,
    total_tokens: 1_000_000 * ONE_TOKEN,
    price_per_token: ONE_VARA / 100,  // 0.01 VARA per token
    min_raise: 500 * ONE_VARA,         // Soft cap: 500 VARA
    max_raise: 10_000 * ONE_VARA,      // Hard cap: 10,000 VARA
    max_per_wallet: 100 * ONE_VARA,    // Max 100 VARA per person
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

### Managing Whitelist

```rust
// Add addresses to whitelist before start
let whitelist = vec![user1, user2, user3];
launchpad.add_to_whitelist(launch_id, whitelist)?;

// Activate the launch
launchpad.start_launch(launch_id)?;
```

### Contributing to a Launch

```rust
// Contribute 50 VARA to get tokens
let tokens = launchpad.contribute(launch_id); // value: 50 * ONE_VARA
// Returns number of tokens purchased
```

### After Launch Ends

```rust
// Anyone can finalize after end time
let status = launchpad.finalize(launch_id)?;

match status {
    LaunchStatus::Succeeded => {
        // Creator withdraws funds
        let amount = launchpad.withdraw_funds(launch_id)?;

        // Contributors claim tokens (respecting vesting if configured)
        let tokens = launchpad.claim_tokens(launch_id)?;
    },
    LaunchStatus::Failed => {
        // Contributors claim refunds
        let refund = launchpad.claim_refund(launch_id)?;
    },
    _ => {}
}
```

## Launch Lifecycle

```
┌─────────────────┐
│    Pending      │ (setup whitelist, configure)
│    (created)    │
└────────┬────────┘
         │ start_launch()
         ▼
┌─────────────────┐
│     Active      │◄───┐ contribute()
│ (time window)   │    │ (during window)
└────────┬────────┘────┘
         │ end_time passed
         ▼
┌─────────────────┐
│   finalize()    │
└────────┬────────┘
    ┌────┴────┐
    │         │
    ▼         ▼
┌───────────┐ ┌───────────┐
│ Succeeded │ │  Failed   │ (min_raise not met)
└─────┬─────┘ └─────┬─────┘
      │             │
      ▼             ▼
┌───────────┐ ┌───────────┐
│ withdraw  │ │  refunds  │
│ + claims  │ │ available │
└───────────┘ └───────────┘
```

## Vesting Integration

When vesting is configured:

```
Launch End → Cliff Period → Linear Vesting → Full Release
    │             │              │              │
    │         no claims     partial claims    all tokens
    │         allowed       available          claimable
```

Users can call `claim_tokens` multiple times as tokens vest.

## Deployment

### Build

```bash
cargo build --release -p launchpad
```

### Constructor Options

- `New`: Initialize with default 2% fee
- `NewWithFee(fee_basis_points)`: Initialize with custom fee

### Deploy via Gear IDEA

1. Upload `target/wasm32-unknown-unknown/release/launchpad.opt.wasm`
2. Upload `target/wasm32-unknown-unknown/release/launchpad.idl`
3. Call constructor `New` or `NewWithFee`

## Security Considerations

- [ ] **Whitelist Enforcement**: Only whitelisted addresses can contribute when enabled
- [ ] **Time Window Validation**: Contributions only during active period
- [ ] **Per-Wallet Limits**: Maximum contribution enforced per address
- [ ] **Soft/Hard Cap Protection**: Min raise required for success, max raise caps total
- [ ] **Refund Guarantee**: Full refunds if minimum not reached
- [ ] **Double-Claim Prevention**: Track claimed amounts per user
- [ ] **Vesting Cliff**: Tokens locked until cliff period ends
- [ ] **Creator Authorization**: Only creator can withdraw funds

## Testing

```bash
cargo test --release -p launchpad
```

## License

MIT OR Apache-2.0
