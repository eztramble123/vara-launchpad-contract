# Deployment Guide

This guide covers deploying the Vara Launchpad Contract to the Vara Network.

## Prerequisites

1. **Rust Toolchain**: Install via [rustup](https://rustup.rs/)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustup update
   ```

2. **WASM Target**:
   ```bash
   rustup target add wasm32v1-none
   ```

3. **Vara Wallet**: Create a wallet at [vara.network](https://vara.network)

4. **VARA Tokens**: Obtain testnet tokens from the faucet

## Building the Contract

### Build Command

```bash
cargo build --release -p launchpad
```

### Output Location

WASM file is generated at:
```
target/wasm32-gear/release/launchpad.opt.wasm
```

### Verify Build

```bash
ls -la target/wasm32-gear/release/launchpad.opt.wasm
```

## Deployment Methods

### Method 1: Gear IDEA (Recommended for Testing)

1. Visit [idea.gear-tech.io](https://idea.gear-tech.io/)
2. Connect your Vara wallet
3. Click "Upload Program"
4. Select `launchpad.opt.wasm`
5. Choose constructor:
   - `New` - Default 2% platform fee
   - `NewWithFee` - Custom fee (enter basis points, e.g., 100 = 1%)
6. Submit transaction
7. Save the Program ID

### Method 2: gcli (Command Line)

```bash
# Install gcli
cargo install gcli

# Deploy with default fee (2%)
gcli program upload ./target/wasm32-gear/release/launchpad.opt.wasm \
    --gas-limit 100000000000 \
    --payload "New"

# Deploy with custom fee (1%)
gcli program upload ./target/wasm32-gear/release/launchpad.opt.wasm \
    --gas-limit 100000000000 \
    --payload "NewWithFee(100)"
```

### Method 3: Programmatic Deployment

```rust
use sails_rs::gclient::GearApi;

async fn deploy_launchpad() -> Result<ActorId, Error> {
    let api = GearApi::dev().await?;

    // Read WASM file
    let wasm = include_bytes!("../target/wasm32-gear/release/launchpad.opt.wasm");

    // Encode constructor
    let payload = "New".encode(); // or "NewWithFee".encode() + fee.encode()

    // Upload and initialize
    let (program_id, _) = api
        .upload_program(
            wasm.to_vec(),
            gclient::now_micros().to_le_bytes(), // salt
            payload,
            100_000_000_000, // gas limit
            0, // value
        )
        .await?;

    Ok(program_id)
}
```

## Constructor Options

### `New`
- Platform fee: 200 basis points (2%)
- Deployer becomes contract owner
- No additional parameters required

### `NewWithFee(fee_basis_points: u16)`
- Custom platform fee
- `fee_basis_points`: Fee in basis points (100 = 1%, 200 = 2%, etc.)
- Maximum: 10000 (100%)

## Post-Deployment Verification

### 1. Query Contract State

```bash
# Check owner
gcli program read <PROGRAM_ID> --method "Launchpad.GetOwner"

# Check pause status
gcli program read <PROGRAM_ID> --method "Launchpad.IsPaused"

# Check launch count
gcli program read <PROGRAM_ID> --method "Launchpad.GetLaunchCount"
```

### 2. Test Transaction

Create a test launch to verify functionality:

```rust
let input = CreateLaunchInput {
    title: "Test Launch".into(),
    description: "Deployment verification".into(),
    token_address: ActorId::zero(), // dummy for test
    total_tokens: 1000,
    price_per_token: 1,
    min_raise: 100,
    max_raise: 1000,
    max_per_wallet: 100,
    start_time: current_block + 100,
    end_time: current_block + 1000,
    whitelist_enabled: false,
    vesting_config: None,
};
```

### 3. Monitor Events

Subscribe to contract events:

```rust
let subscription = api.subscribe_to_program_events(program_id).await?;

while let Some(event) = subscription.next().await {
    println!("Event: {:?}", event);
}
```

## Network Configuration

### Testnet

| Property | Value |
|----------|-------|
| RPC | `wss://testnet.vara.network` |
| Explorer | [testnet.vara.network](https://testnet.vara.network) |
| Faucet | Available in Discord |

### Mainnet

| Property | Value |
|----------|-------|
| RPC | `wss://rpc.vara.network` |
| Explorer | [vara.network](https://vara.network) |

## Gas Estimation

Before transactions, estimate gas:

```rust
let gas = api.calculate_gas(
    program_id,
    payload,
    value,
    true, // allow_other_panic
).await?;

// Add buffer for safety
let gas_limit = gas.min_limit.saturating_mul(12).saturating_div(10);
```

## Upgrading

Vara contracts are immutable. For upgrades:

1. Deploy new contract version
2. Migrate data if needed (via queries + new transactions)
3. Update frontend/integrations to new address
4. Communicate migration plan to users

## Troubleshooting

### Common Issues

| Issue | Solution |
|-------|----------|
| Out of Gas | Increase gas limit |
| Insufficient Balance | Add VARA to wallet |
| Invalid Payload | Verify SCALE encoding |
| Program Trapped | Check constructor parameters |
| Build Fails | Run `cargo clean` and rebuild |

### Debug Mode

Enable debug logging in contracts:

```rust
gstd::debug!("Debug: {:?}", value);
```

View logs in block explorer or gcli output.

### WASM Not Found

If WASM file not generated:

1. Check build output for errors
2. Verify Rust toolchain version
3. Run `cargo clean && cargo build --release -p launchpad`

## Security Checklist Before Mainnet

- [ ] All tests pass
- [ ] Code reviewed by independent party
- [ ] No debug statements in production code
- [ ] Correct fee parameters configured
- [ ] Owner address verified
- [ ] Deploy first to testnet
- [ ] Test all critical paths on testnet
