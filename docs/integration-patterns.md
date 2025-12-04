# Integration Patterns

This guide describes optional integration patterns between contracts in this library.

## Contract Dependencies

While each contract is standalone, they can be composed for enhanced functionality:

```
┌─────────────────┐     ┌─────────────────┐
│  Access Control │◄────│    Any Contract │
│     (RBAC)      │     │  (role checks)  │
└─────────────────┘     └─────────────────┘
         ▲
         │
┌────────┴────────┐     ┌─────────────────┐
│    Reputation   │◄────│    Governance   │
│    (scoring)    │     │  (vote weight)  │
└─────────────────┘     └─────────────────┘
```

## Integration Examples

### Access Control + Any Contract

Use Access Control for role management in other contracts:

```rust
// In your contract
fn check_admin(&self, account: ActorId) -> bool {
    // Call access control contract
    let result: bool = msg::send_for_reply(
        access_control_id,
        AccessControlAction::HasRole {
            role: ADMIN_ROLE,
            account,
        },
        0,
        0,
    ).await.unwrap();
    result
}

fn restricted_operation(&mut self) -> Result<(), Error> {
    let caller = msg::source();
    if !self.check_admin(caller) {
        return Err(Error::Unauthorized);
    }
    // Proceed with operation
    Ok(())
}
```

### Reputation + Governance

Use reputation scores as vote weights:

```rust
// In governance contract
fn get_vote_weight(&self, voter: ActorId) -> u128 {
    // Get reputation score from Reputation contract
    let reputation: i64 = msg::send_for_reply(
        reputation_contract_id,
        ReputationAction::GetScore { user: voter },
        0,
        0,
    ).await.unwrap();

    // Convert to vote weight (minimum 1)
    reputation.max(0) as u128 + 1
}
```

### Launchpad + Vesting

Auto-create vesting schedules for token purchases:

```rust
// In launchpad after successful claim
async fn create_vesting_for_purchase(
    &self,
    buyer: ActorId,
    tokens: u128,
    config: &VestingConfig,
) -> Result<u64, Error> {
    let schedule_id: u64 = msg::send_for_reply(
        vesting_contract_id,
        VestingAction::CreateSchedule {
            beneficiary: buyer,
            amount: tokens,
            start_block: config.start_block,
            cliff_duration: config.cliff_duration,
            vesting_duration: config.vesting_duration,
            revocable: false,
        },
        0,
        0,
    ).await?;

    Ok(schedule_id)
}
```

### Crowdfunding + Reputation

Award badges to contributors:

```rust
// After contribution
async fn award_backer_badge(&self, contributor: ActorId, amount: u128) {
    let badge_id = if amount >= 1000 * DECIMALS {
        WHALE_BACKER_BADGE
    } else if amount >= 100 * DECIMALS {
        GOLD_BACKER_BADGE
    } else {
        BRONZE_BACKER_BADGE
    };

    let _ = msg::send(
        reputation_contract_id,
        ReputationAction::AwardBadge {
            user: contributor,
            badge_id,
        },
        0,
    );
}
```

### Escrow + Reputation

Update reputation based on deal outcomes:

```rust
// After successful deal completion
async fn update_deal_reputation(&self, deal: &Deal) {
    // Increase seller reputation
    msg::send(
        reputation_contract_id,
        ReputationAction::AddReputation {
            user: deal.seller,
            amount: 10,
            reason: "Completed escrow deal".into(),
        },
        0,
    );

    // Increase buyer reputation
    msg::send(
        reputation_contract_id,
        ReputationAction::AddReputation {
            user: deal.buyer,
            amount: 5,
            reason: "Released escrow payment".into(),
        },
        0,
    );
}
```

## Cross-Contract Communication

### Synchronous Queries

For read-only operations, use synchronous message passing:

```rust
async fn query_contract<T: Decode>(
    target: ActorId,
    payload: impl Encode,
) -> Result<T, Error> {
    msg::send_for_reply_as(target, payload, 0, 0)
        .await
        .map_err(|_| Error::CrossContractCallFailed)
}
```

### Asynchronous Actions

For state-changing operations with callbacks:

```rust
async fn call_with_callback<T: Decode>(
    target: ActorId,
    payload: impl Encode,
    value: u128,
) -> Result<T, Error> {
    let result = msg::send_for_reply_as(target, payload, value, 0)
        .await
        .map_err(|_| Error::CrossContractCallFailed)?;

    // Handle callback
    Ok(result)
}
```

## Best Practices

### 1. Loose Coupling

Keep integrations optional:

```rust
pub struct MyContract {
    // Optional integration
    reputation_contract: Option<ActorId>,
}

fn award_reputation(&self, user: ActorId, amount: i64) {
    if let Some(rep_contract) = self.reputation_contract {
        let _ = msg::send(rep_contract, /* ... */, 0);
    }
}
```

### 2. Error Handling

Don't fail primary operations on integration failures:

```rust
// Award badge but don't fail if it errors
let _ = self.award_reputation_badge(user, badge_id).await;

// Continue with primary operation
self.complete_action()?;
```

### 3. Gas Management

Account for cross-contract call gas:

```rust
const CROSS_CONTRACT_GAS: u64 = 10_000_000_000;

fn calculate_total_gas(base_gas: u64, cross_calls: u64) -> u64 {
    base_gas + (cross_calls * CROSS_CONTRACT_GAS)
}
```

### 4. Event Consistency

Emit events for cross-contract operations:

```rust
fn emit_integration_event(&self, action: &str, target: ActorId) {
    let _ = self.emit_event(MyEvent::CrossContractCall {
        action: action.into(),
        target,
        timestamp: exec::block_timestamp(),
    });
}
```

## Deployment Considerations

### Contract Dependencies

Deploy in order of dependency:
1. Access Control (foundation)
2. Reputation (used by others)
3. Vesting (used by launchpad)
4. Other contracts

### Configuration

Store integrated contract addresses:

```rust
pub struct Config {
    access_control: Option<ActorId>,
    reputation: Option<ActorId>,
    vesting: Option<ActorId>,
}

fn init_integrations(&mut self, config: Config) {
    self.config = config;
}
```

### Testing Integrations

Test with deployed contracts:

```rust
#[tokio::test]
async fn test_integration() {
    let system = System::new();

    // Deploy both contracts
    let access_control = deploy_access_control(&system);
    let my_contract = deploy_my_contract(&system, access_control);

    // Test integrated behavior
    my_contract.call_with_access_control();
}
```
