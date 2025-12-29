# VFT Token Integration Guide

## Overview

The Vara Launchpad uses Gear Foundation's standard VFT (Vara Fungible Token) implementation for automatic token deployment. This ensures compatibility with the Vara ecosystem including DEXes, bridges, and wallets.

## Standard VFT Code ID

The Gear Foundation provides a standard VFT implementation with role-based access control:

- **Mainnet/Testnet Code ID**: `0x81663df58f48684923777cd8cf281bfd2e4ee427926abc52a1fcf4ecd41be7ad`
- **Repository**: https://github.com/gear-foundation/standards/tree/master/extended-vft

## Setup Process

### 1. Deploy the Launchpad Contract

```bash
# Build the launchpad
cargo build --release -p vara-launchpad

# Deploy via Gear IDEA or CLI
# The deploying account becomes the owner
```

### 2. Set the VFT Code ID

After deploying the launchpad, set the VFT code ID so the contract can deploy tokens:

```javascript
// Using the standard Gear VFT code ID
const VFT_CODE_ID = "0x81663df58f48684923777cd8cf281bfd2e4ee427926abc52a1fcf4ecd41be7ad";

await launchpad.setVftCodeId(VFT_CODE_ID);
```

### 3. Create a Launch with Token Deployment

When creating a launch, the launchpad will automatically deploy a new VFT token:

```javascript
const launch = await launchpad.createLaunch({
    // Token parameters
    token_name: "My Token",
    token_symbol: "MTK",
    
    // Launch parameters  
    title: "My Token Fair Launch",
    description: "Community-driven token launch",
    total_tokens: 1_000_000_000, // 1 billion tokens
    price_per_token: 100, // Price in smallest VARA units
    min_raise: 1000,
    max_raise: 10000,
    max_per_wallet: 100,
    start_time: currentBlock + 100,
    end_time: currentBlock + 10000,
    whitelist_enabled: false,
    vesting_config: null
});
```

## Token Deployment Flow

1. **Launch Creation**: User calls `create_launch()` with token parameters
2. **Token Deployment**: Launchpad deploys a new VFT instance with:
   - Name and symbol from input
   - Total supply minted to launchpad
   - Launchpad as initial admin/minter/burner
3. **Token Ready**: Token is immediately available for the launch
4. **Fair Distribution**: On success, tokens are transferred to buyers at fixed price

## VFT Token Features

The deployed tokens have full VFT functionality:

- **Transfer**: Direct token transfers between accounts
- **Approve/TransferFrom**: Allowance system for DEX integration  
- **Role Management**: Admin, minter, burner roles
- **Standard Queries**: balanceOf, totalSupply, decimals, etc.

## Security Considerations

### Role Management
- The launchpad becomes the initial admin of deployed tokens
- After successful launch, consider transferring admin role to a DAO or burning it
- Minter role should be revoked after initial supply is created

### Token Supply
- All tokens are minted at deployment to the launchpad
- No additional minting is possible after launch
- This ensures a fair, fixed supply distribution

## DEX Integration

Deployed tokens are automatically compatible with Vara DEXes:

```javascript
// After successful launch, add liquidity
const tokenAddress = launch.token_address;
const varaAmount = launch.total_raised * 0.8; // 80% of raised funds
const tokenAmount = launch.total_tokens * 0.2; // 20% of tokens

// Add to DEX (example)
await dex.addLiquidity(
    tokenAddress,
    VARA_ADDRESS,
    tokenAmount,
    varaAmount
);
```

## Troubleshooting

### "VFT code ID not set"
The launchpad owner must call `set_vft_code_id()` with a valid VFT code ID before launches can be created.

### Token deployment fails
Ensure:
- The VFT code ID is valid and exists on-chain
- The launchpad has enough gas for deployment
- Token name/symbol are not empty

### Cannot claim tokens
Verify:
- The launch succeeded (min raise met)
- The claim period has started
- User has unclaimed tokens

## Example: Complete Launch Flow

```javascript
// 1. Setup (one time, by owner)
await launchpad.setVftCodeId(VFT_CODE_ID);

// 2. Create launch (by any user)
const launchId = await launchpad.createLaunch({
    token_name: "Community Token",
    token_symbol: "COMM",
    title: "Community Token Launch",
    description: "Fair launch for everyone",
    total_tokens: 1_000_000_000,
    price_per_token: 100,
    min_raise: 5000,
    max_raise: 50000,
    max_per_wallet: 500,
    start_time: currentBlock + 1000,
    end_time: currentBlock + 10000,
    whitelist_enabled: false,
    vesting_config: null
});

// 3. Start launch
await launchpad.startLaunch(launchId);

// 4. Users contribute
await launchpad.contribute(launchId, { value: 100 });

// 5. Finalize after end time
await launchpad.finalize(launchId);

// 6. Success path - claim tokens
await launchpad.claimTokens(launchId);

// 7. Creator withdraws funds
await launchpad.withdrawFunds(launchId);
```

## Resources

- [Gear VFT Standard](https://github.com/gear-foundation/standards/tree/master/extended-vft)
- [Vara Network Docs](https://docs.vara.network)
- [Gear IDEA Portal](https://idea.gear-tech.io)