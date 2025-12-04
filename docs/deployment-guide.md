# Deployment Guide

This guide covers deploying contracts from the Vara Contract Template Library to the Vara Network.

## Prerequisites

1. **Rust Toolchain**: Install via [rustup](https://rustup.rs/)
2. **WASM Target**: `rustup target add wasm32v1-none`
3. **Vara Wallet**: Create a wallet at [vara.network](https://vara.network)
4. **VARA Tokens**: Obtain testnet tokens from the faucet

## Building Contracts

### Build All Contracts

```bash
cargo build --release
```

### Build Individual Contract

```bash
# Example: Build escrow contract
cargo build --release -p escrow
```

### Output Location

WASM files are generated at:
```
target/wasm32v1-none/release/<contract_name>.opt.wasm
```

## Deployment Methods

### Using Gear Idea (Recommended for Testing)

1. Visit [idea.gear-tech.io](https://idea.gear-tech.io/)
2. Connect your Vara wallet
3. Click "Upload Program"
4. Select the `.opt.wasm` file
5. Configure initialization parameters
6. Submit transaction

### Using gcli (Command Line)

```bash
# Install gcli
cargo install gcli

# Deploy contract
gcli program upload ./target/wasm32v1-none/release/escrow.opt.wasm \
    --gas-limit 100000000000 \
    --value 0
```

### Using Sails Client (Programmatic)

```rust
use sails_rs::gclient::GearApi;

async fn deploy() {
    let api = GearApi::dev().await.unwrap();

    let program_id = api
        .upload_program(
            include_bytes!("../target/wasm32v1-none/release/escrow.opt.wasm"),
            b"salt",
            init_payload,
            gas_limit,
            value,
        )
        .await
        .unwrap();
}
```

## Contract Initialization

Each contract has specific initialization parameters:

### Access Control
```
Constructor: New
Parameters: None (deployer becomes admin)
```

### Escrow
```
Constructor: New | NewWithFee(fee_basis_points)
Parameters:
  - fee_basis_points: u16 (optional, default 100 = 1%)
```

### Vesting
```
Constructor: New
Parameters: None (deployer becomes owner)
```

### Crowdfunding
```
Constructor: New | NewWithFee(fee_basis_points)
Parameters:
  - fee_basis_points: u16 (optional, default 100 = 1%)
```

### Voting
```
Constructor: New | NewWithConfig(config)
Parameters:
  - config: GovernanceConfig (optional)
    - voting_period: u32
    - quorum: u128
    - proposal_threshold: u128
    - execution_delay: u32
```

### Reputation
```
Constructor: New
Parameters: None (deployer becomes owner and updater)
```

### Lending
```
Constructor: New | NewWithConfig(config)
Parameters:
  - config: LendingConfig (optional)
    - collateral_ratio: u16
    - liquidation_threshold: u16
    - liquidation_bonus: u16
    - interest_rate: u16
    - blocks_per_year: u32
```

### Launchpad
```
Constructor: New | NewWithFee(fee_basis_points)
Parameters:
  - fee_basis_points: u16 (optional, default 200 = 2%)
```

## Post-Deployment Verification

1. **Check Program State**: Query the contract to verify initialization
2. **Test Transactions**: Execute test transactions on testnet
3. **Monitor Events**: Subscribe to contract events

## Network Configuration

### Testnet
- RPC: `wss://testnet.vara.network`
- Explorer: [testnet.vara.network](https://testnet.vara.network)

### Mainnet
- RPC: `wss://rpc.vara.network`
- Explorer: [vara.network](https://vara.network)

## Gas Estimation

Use the `calculate_gas` API to estimate gas before transactions:

```rust
let gas = api.calculate_gas(
    program_id,
    payload,
    value,
    true, // allow_other_panic
).await?;
```

## Upgrading Contracts

Vara contracts are immutable once deployed. For upgrades:

1. Deploy new contract version
2. Migrate state if needed
3. Update frontend/integrations to new address
4. Consider implementing proxy patterns for upgradability

## Troubleshooting

### Common Issues

1. **Out of Gas**: Increase gas limit
2. **Insufficient Balance**: Add VARA to wallet
3. **Invalid Payload**: Verify SCALE encoding
4. **Program Trapped**: Check initialization parameters

### Debug Mode

Enable debug logging in contracts:
```rust
gstd::debug!("Debug message: {:?}", value);
```

View logs in block explorer or gcli output.
