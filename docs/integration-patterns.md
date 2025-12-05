# Integration Patterns

This guide covers integration patterns for the Vara Launchpad Contract.

## Frontend Integration

### JavaScript/TypeScript with Sails

```typescript
import { GearApi, decodeAddress } from '@gear-js/api';
import { Sails } from 'sails-js';

// Connect to Vara Network
const api = await GearApi.create({ providerAddress: 'wss://testnet.vara.network' });

// Load IDL
const idl = await fetch('/launchpad.idl').then(r => r.text());
const sails = new Sails(api);
sails.parseIdl(idl);

// Connect to deployed contract
const programId = '0x...';
sails.setProgramId(programId);

// Query active launches
const activeLaunches = await sails.services.Launchpad.queries.GetActiveLaunches();

// Create a launch
const input = {
  title: 'My Token Sale',
  description: 'Fair launch',
  token_address: decodeAddress('kG...'),
  total_tokens: BigInt(1000000) * BigInt(10**12),
  price_per_token: BigInt(10**9), // 0.001 VARA
  min_raise: BigInt(100) * BigInt(10**12),
  max_raise: BigInt(1000) * BigInt(10**12),
  max_per_wallet: BigInt(100) * BigInt(10**12),
  start_time: currentBlock + 1000,
  end_time: currentBlock + 10000,
  whitelist_enabled: false,
  vesting_config: null,
};

const tx = sails.services.Launchpad.functions.CreateLaunch(input);
await tx.signAndSend(account);

// Contribute to a launch
const contributeTx = sails.services.Launchpad.functions.Contribute(launchId);
await contributeTx.withValue(BigInt(50) * BigInt(10**12)).signAndSend(account);
```

### Event Subscription

```typescript
// Subscribe to all launchpad events
api.gearEvents.subscribeToGearEvent('UserMessageSent', ({ data }) => {
  if (data.source.eq(programId)) {
    const event = sails.services.Launchpad.events.decode(data.payload);

    switch (event.type) {
      case 'LaunchCreated':
        console.log('New launch:', event.launch_id);
        break;
      case 'Contributed':
        console.log('Contribution:', event.amount);
        break;
      case 'LaunchSucceeded':
        console.log('Launch succeeded:', event.launch_id);
        break;
    }
  }
});
```

## Backend Integration

### Rust Client

```rust
use sails_rs::gclient::GearApi;
use launchpad_client::LaunchpadClient;

async fn interact_with_launchpad() -> Result<(), Error> {
    // Connect to network
    let api = GearApi::init(Some("wss://testnet.vara.network")).await?;

    // Create client
    let program_id = "0x...".parse()?;
    let client = LaunchpadClient::new(&api, program_id);

    // Query launches
    let active = client.get_active_launches().await?;

    // Create launch
    let input = CreateLaunchInput {
        title: "My Launch".into(),
        // ... other fields
    };
    let launch_id = client.create_launch(input).await?;

    // Contribute
    let tokens = client.contribute(launch_id, value).await?;

    Ok(())
}
```

### Event Indexer

```rust
use sails_rs::gclient::GearApi;
use futures::StreamExt;

async fn index_events(api: &GearApi, program_id: ProgramId) {
    let mut subscription = api.subscribe_to_all_messages().await.unwrap();

    while let Some(message) = subscription.next().await {
        if message.source() == program_id {
            match decode_event(&message.payload()) {
                LaunchpadEvent::LaunchCreated { launch_id, creator, .. } => {
                    // Index in database
                    db.insert_launch(launch_id, creator).await;
                }
                LaunchpadEvent::Contributed { launch_id, contributor, amount, .. } => {
                    // Update contribution records
                    db.record_contribution(launch_id, contributor, amount).await;
                }
                // Handle other events...
            }
        }
    }
}
```

## VFT Token Integration

### Token Deposit Flow

For projects that want to pre-deposit tokens:

```rust
// Creator deposits tokens before launch
async fn deposit_tokens(
    vft_client: &VftClient,
    launchpad_id: ProgramId,
    amount: u128,
) -> Result<(), Error> {
    // Approve launchpad to spend tokens
    vft_client.approve(launchpad_id, amount).await?;

    // Transfer tokens to launchpad
    vft_client.transfer(launchpad_id, amount).await?;

    // Mark as deposited (optional, for UI)
    launchpad_client.mark_tokens_deposited(launch_id).await?;

    Ok(())
}
```

### Token Distribution

The contract tracks token allocations. For actual VFT transfers, integrate with your token contract:

```typescript
// After successful claim
async function distributeTokens(
  vftContract: VftClient,
  claimer: Address,
  amount: bigint,
): Promise<void> {
  // Transfer tokens from treasury/launchpad to claimer
  await vftContract.transfer(claimer, amount);
}
```

## Multi-Launch Platform

### Centralized Launch Management

```typescript
class LaunchpadManager {
  constructor(
    private sails: Sails,
    private db: Database,
  ) {}

  async createLaunch(params: LaunchParams): Promise<LaunchId> {
    // Validate params
    this.validateParams(params);

    // Create on-chain
    const launchId = await this.sails.services.Launchpad
      .functions.CreateLaunch(params.toInput())
      .signAndSend(params.creator);

    // Store metadata off-chain
    await this.db.storeLaunchMetadata(launchId, {
      images: params.images,
      socials: params.socials,
      whitepaper: params.whitepaper,
    });

    return launchId;
  }

  async getLaunchWithMetadata(launchId: LaunchId): Promise<LaunchDetails> {
    // Fetch on-chain data
    const launch = await this.sails.services.Launchpad
      .queries.GetLaunch(launchId);

    // Fetch off-chain metadata
    const metadata = await this.db.getLaunchMetadata(launchId);

    return { ...launch, ...metadata };
  }
}
```

### Batch Operations

```typescript
// Batch whitelist updates
async function batchWhitelist(
  sails: Sails,
  launchId: LaunchId,
  addresses: string[],
  batchSize: number = 100,
): Promise<void> {
  for (let i = 0; i < addresses.length; i += batchSize) {
    const batch = addresses.slice(i, i + batchSize);
    await sails.services.Launchpad
      .functions.AddToWhitelist(launchId, batch)
      .signAndSend(creator);
  }
}
```

## Analytics Integration

### Event Processing Pipeline

```typescript
class LaunchpadEventProcessor {
  async processEvent(event: LaunchpadEvent): Promise<void> {
    switch (event.type) {
      case 'LaunchCreated':
        await this.analytics.trackLaunchCreated({
          launchId: event.launch_id,
          creator: event.creator,
          totalTokens: event.total_tokens,
          pricePerToken: event.price_per_token,
        });
        break;

      case 'Contributed':
        await this.analytics.trackContribution({
          launchId: event.launch_id,
          contributor: event.contributor,
          amount: event.amount,
          tokensReceived: event.tokens_purchased,
        });
        break;

      case 'LaunchSucceeded':
        await this.analytics.trackLaunchSuccess({
          launchId: event.launch_id,
          totalRaised: event.total_raised,
        });
        break;
    }
  }
}
```

### Dashboard Metrics

```typescript
interface LaunchpadMetrics {
  totalLaunches: number;
  activeLaunches: number;
  successfulLaunches: number;
  totalRaised: bigint;
  totalContributors: number;
  platformFees: bigint;
}

async function getMetrics(sails: Sails): Promise<LaunchpadMetrics> {
  const [launchCount, fees] = await Promise.all([
    sails.services.Launchpad.queries.GetLaunchCount(),
    sails.services.Launchpad.queries.GetAccumulatedFees(),
  ]);

  return {
    totalLaunches: launchCount,
    // ... aggregate from indexed events
    platformFees: fees,
  };
}
```

## Error Handling

### Contract Errors

```typescript
function handleContractError(error: any): UserFriendlyError {
  const errorType = parseContractError(error);

  switch (errorType) {
    case 'Unauthorized':
      return { message: 'You are not authorized to perform this action' };
    case 'NotFound':
      return { message: 'Launch not found' };
    case 'InvalidState':
      return { message: 'This action is not available in the current state' };
    default:
      return { message: 'An unexpected error occurred' };
  }
}
```

### Retry Logic

```typescript
async function contributeWithRetry(
  sails: Sails,
  launchId: LaunchId,
  amount: bigint,
  maxRetries: number = 3,
): Promise<ContributionResult> {
  for (let attempt = 0; attempt < maxRetries; attempt++) {
    try {
      return await sails.services.Launchpad
        .functions.Contribute(launchId)
        .withValue(amount)
        .signAndSend(account);
    } catch (error) {
      if (isTransientError(error) && attempt < maxRetries - 1) {
        await delay(1000 * (attempt + 1));
        continue;
      }
      throw error;
    }
  }
}
```

## Best Practices

### Gas Management

```typescript
async function estimateAndExecute(
  sails: Sails,
  tx: Transaction,
): Promise<Result> {
  const gasInfo = await tx.calculateGas();
  const gasLimit = gasInfo.min_limit * 12n / 10n; // 20% buffer

  return tx.withGas(gasLimit).signAndSend(account);
}
```

### State Consistency

```typescript
async function safeContribute(
  sails: Sails,
  launchId: LaunchId,
  amount: bigint,
): Promise<void> {
  // Fetch current state
  const launch = await sails.services.Launchpad.queries.GetLaunch(launchId);

  // Validate
  if (launch.status !== 'Active') {
    throw new Error('Launch is not active');
  }

  const currentBlock = await getCurrentBlock();
  if (currentBlock < launch.start_time || currentBlock > launch.end_time) {
    throw new Error('Outside contribution window');
  }

  // Execute
  await sails.services.Launchpad
    .functions.Contribute(launchId)
    .withValue(amount)
    .signAndSend(account);
}
```
