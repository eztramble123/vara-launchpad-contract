# Security Checklist

Security considerations and audit recommendations for the Vara Launchpad Contract.

## General Security Principles

### Actor Model Security

Vara's actor model provides inherent isolation, but be aware of:

- [ ] Message ordering is not guaranteed
- [ ] Async operations can interleave
- [ ] State must be consistent between messages
- [ ] External calls can fail

### Integer Overflow/Underflow

The contract uses safe arithmetic throughout:

- [x] `saturating_*` operations for arithmetic
- [x] `checked_*` operations where overflow must error
- [x] Input amounts validated against type bounds
- [x] Vesting calculations use scaled math to prevent precision loss

### Access Control

- [x] Caller identity verified with `gstd::msg::source()`
- [x] Creator-only operations: `start_launch`, `withdraw_funds`, `cancel_launch`
- [x] Owner-only operations: `pause`, `resume`, `withdraw_fees`
- [x] Anyone can call: `finalize`, `claim_tokens`, `claim_refund`

### Input Validation

- [x] All user inputs validated
- [x] Non-empty title required
- [x] Positive amounts required (tokens, price, limits)
- [x] Start time must be in future
- [x] Start time must be before end time
- [x] Min raise <= max raise
- [x] Max raise <= total_tokens * price_per_token

## Contract-Specific Checklist

### State Machine

- [x] Clear state transitions defined
- [x] Invalid state transitions rejected
- [x] `Pending` → `Active` only via `start_launch`
- [x] `Active` → `Ended` when time expires or fully subscribed
- [x] `Ended` → `Succeeded` or `Failed` based on min_raise
- [x] `Cancelled` only allowed for `Pending` (by creator) or any state (by owner)

### Contribution Logic

- [x] Only allowed during `Active` state
- [x] Only within time window (start_time <= current <= end_time)
- [x] Whitelist enforced when enabled
- [x] Per-wallet limits enforced
- [x] Excess contributions refunded automatically
- [x] Contribution too small for 1 token rejected
- [x] Token purchase calculation uses safe math
- [x] Contributors tracked for batch operations

### Token Economics

- [x] `total_raised` preserved (not wiped on withdrawal)
- [x] `funds_withdrawn` flag prevents double withdrawal
- [x] Platform fee calculated correctly (basis points / 10000)
- [x] Fee accumulated, not transferred immediately
- [x] Tokens purchased tracked separately from contributions

### Refund System

- [x] Refunds only for `Failed` or `Cancelled` launches
- [x] Contribution removed on refund (prevents double-claim)
- [x] All contributors can be enumerated
- [x] Refund transfers use safe native transfer

### Token Claims

- [x] Claims only for `Succeeded`/`DistributionPending` launches
- [x] Claimed amount tracked per user
- [x] Vesting calculation accounts for cliff period
- [x] Vesting uses scaled math for precision
- [x] Multiple claims allowed as tokens vest
- [x] Cannot claim more than purchased

### Vesting

- [x] Cliff period enforced (no claims before cliff ends)
- [x] Linear vesting calculation is accurate
- [x] Full vesting after vesting_end block
- [x] Scale factor (10^12) prevents rounding errors

### Pause Mechanism

- [x] Only owner can pause/resume
- [x] `create_launch` blocked when paused
- [x] `contribute` blocked when paused
- [x] Claims and refunds still work when paused (user protection)

## CEI Pattern Compliance

All state-changing operations follow Checks-Effects-Interactions:

```rust
// 1. Checks
if caller != launch.creator {
    return Err(ContractError::Unauthorized);
}

// 2. Effects (state changes)
launch.funds_withdrawn = true;
s.accumulated_fees = s.accumulated_fees.saturating_add(fee);

// 3. Interactions (external calls)
transfer_native(caller, amount_to_creator)?;
```

## Testing Requirements

### Unit Tests

- [x] Contract initialization
- [x] Launch creation with valid parameters
- [x] Launch start
- [x] Contributions within limits
- [x] Whitelist enforcement
- [x] Finalization (success path)
- [x] Finalization (failure path)
- [x] Token claims
- [x] Refund claims
- [x] Fund withdrawal
- [x] Unauthorized access rejection
- [x] Per-wallet limit enforcement
- [x] Pause/resume functionality
- [x] Platform fee withdrawal
- [x] Query operations
- [x] Full lifecycle test

### Edge Cases Tested

- [x] Zero amounts rejected
- [x] Contribution at exactly max_per_wallet
- [x] Over-limit contribution rejected
- [x] Non-whitelisted user rejected
- [x] Contribution outside time window rejected
- [x] Unauthorized withdraw rejected
- [x] Cancel by non-creator rejected (except owner)

## Pre-Deployment Checklist

### Code Review

- [ ] Independent review completed
- [ ] All TODOs resolved
- [ ] Dead code removed
- [ ] Debug statements removed
- [ ] No hardcoded addresses

### Configuration

- [ ] Fee parameters are reasonable (default 2%)
- [ ] Time parameters appropriate for network block times
- [ ] Owner address verified
- [ ] Initial state is correct

### Documentation

- [x] API documentation complete
- [x] Integration guide available
- [x] Known limitations documented
- [x] Events documented for indexers

## Post-Deployment Monitoring

### Event Monitoring

Monitor these events for anomalies:

| Event | What to Watch |
|-------|---------------|
| `LaunchCreated` | Unusual parameters |
| `Contributed` | Large contributions |
| `LaunchFailed` | Unexpected failures |
| `FundsWithdrawn` | Verify amounts match expectations |
| `Paused` | Emergency activation |

### State Monitoring

- [ ] Regular state snapshots
- [ ] Balance reconciliation
- [ ] Track accumulated fees
- [ ] Monitor active launch count

## Emergency Procedures

### Pause Mechanism

The contract includes pause functionality:

```rust
// Owner can pause
launchpad.pause()?;

// All new launches blocked
// All contributions blocked
// Claims and refunds still work
```

### Recovery Steps

1. Pause affected functionality
2. Analyze root cause
3. Document impact
4. If critical: encourage users to claim refunds
5. Deploy fixed version if needed
6. Communicate with users

## Known Limitations

### Static Mut References

The `static mut` pattern used for storage generates Rust 2024 compatibility warnings. This is the standard pattern for Vara contracts and is safe in the single-threaded WASM environment.

### VFT Token Transfers

Token claims track amounts but don't execute actual VFT transfers. Integration with VFT contracts requires:

- Async message to VFT contract
- Gas reservation for callback
- Error handling for failed transfers
- The `TokenTransferFailed` event is emitted for retry scenarios

### Contract Immutability

Contracts are immutable once deployed. Plan for:

- State migration strategies
- Version management
- User communication for upgrades

## Audit Recommendations

For production deployment, we recommend:

1. **Independent Audit**: Professional security audit by Vara-experienced auditors
2. **Formal Verification**: Consider formal verification for critical paths
3. **Bug Bounty**: Establish bug bounty program post-launch
4. **Staged Rollout**: Deploy to testnet, then limited mainnet, then full launch

## Resources

- [Vara Security Documentation](https://wiki.vara.network/docs/security)
- [Sails-RS Best Practices](https://github.com/gear-tech/sails)
- [Smart Contract Audit Checklist](https://consensys.github.io/smart-contract-best-practices/)
