# Contributing to Vara Launchpad Contract

Thank you for your interest in contributing! This document provides guidelines for contributing to the Vara Launchpad Contract.

## Getting Started

### Prerequisites

- Rust 1.88.0 or later
- WASM target: `rustup target add wasm32v1-none`
- Git

### Setup

1. Clone the repository:
   ```bash
   git clone https://github.com/your-org/vara-launchpad-contract.git
   cd vara-launchpad-contract
   ```

2. Build the project:
   ```bash
   cargo build --release -p launchpad
   ```

3. Run tests:
   ```bash
   cargo test --release -p launchpad
   ```

## Project Structure

```
vara-launchpad-contract/
├── contracts/launchpad/
│   ├── app/src/lib.rs      # Core contract logic
│   ├── src/lib.rs          # WASM entry point
│   ├── client/             # Client library
│   ├── tests/gtest.rs      # Integration tests
│   └── build.rs            # Build script
├── shared/                 # Shared types and utilities
└── docs/                   # Documentation
```

## Development Workflow

### Making Changes

1. Create a feature branch:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. Make your changes following the code style guidelines below.

3. Add tests for new functionality.

4. Ensure all tests pass:
   ```bash
   cargo test --release -p launchpad
   ```

5. Run clippy for linting:
   ```bash
   cargo clippy --release -p launchpad
   ```

6. Format your code:
   ```bash
   cargo fmt
   ```

### Code Style

- Follow Rust naming conventions
- Use `saturating_*` or `checked_*` for all arithmetic operations
- Add doc comments for public items
- Keep functions focused and small
- Use meaningful variable names

### Contract Development Guidelines

#### State Management

```rust
// Always update state BEFORE external calls (CEI pattern)
launch.funds_withdrawn = true;  // Update state first
transfer_native(caller, amount)?;  // Then make external call
```

#### Error Handling

```rust
// Use ContractError for all errors
if caller != launch.creator {
    return Err(ContractError::Unauthorized);
}

// Use helper methods for common error patterns
if input.title.is_empty() {
    return Err(ContractError::invalid_input("Title cannot be empty"));
}
```

#### Events

```rust
// Emit events for all state changes
self.emit_event(LaunchpadEvent::LaunchCreated {
    launch_id,
    creator,
    // ... include relevant data
});
```

### Testing Guidelines

#### Test Structure

```rust
#[test]
fn test_feature_name() {
    let system = setup_system();
    let program = deploy_contract(&system);

    // Setup
    // ...

    // Action
    let msg_id = program.send_bytes(/* ... */);
    let result = system.run_next_block();

    // Assert
    assert!(result.succeed.contains(&msg_id), "Expected success");
}
```

#### Test Coverage

- Test happy paths
- Test error conditions
- Test edge cases (zero amounts, max values)
- Test authorization checks
- Test state transitions

### Adding New Features

1. **Plan the feature**: Consider state machine implications
2. **Update types**: Add new types to `shared/src/types.rs` if needed
3. **Implement logic**: Add methods to `LaunchpadService`
4. **Add events**: Create events for state changes
5. **Write tests**: Comprehensive test coverage
6. **Update docs**: Update README and relevant documentation

### Example: Adding a New Method

```rust
// 1. Add the method to LaunchpadService
#[export(unwrap_result)]
pub fn new_feature(&mut self, launch_id: Id, param: Type) -> Result<ReturnType, ContractError> {
    let s = storage_mut();
    let caller = gstd::msg::source();

    // Validate
    let launch = s.launches.get_mut(&launch_id)
        .ok_or(ContractError::NotFound)?;

    if caller != launch.creator {
        return Err(ContractError::Unauthorized);
    }

    // Update state
    // ...

    // Emit event
    self.emit_event(LaunchpadEvent::NewFeature { launch_id });

    Ok(result)
}

// 2. Add event variant
pub enum LaunchpadEvent {
    // ... existing events
    NewFeature { launch_id: Id },
}

// 3. Update SailsEvent impl
impl sails_rs::SailsEvent for LaunchpadEvent {
    fn encoded_event_name(&self) -> &'static [u8] {
        match self {
            // ... existing matches
            LaunchpadEvent::NewFeature { .. } => b"NewFeature",
        }
    }
}

// 4. Add test
#[test]
fn test_new_feature() {
    // ... test implementation
}
```

## Pull Request Process

1. Ensure all tests pass
2. Update documentation if needed
3. Add a clear PR description explaining:
   - What the change does
   - Why it's needed
   - How it was tested
4. Request review from maintainers

### PR Checklist

- [ ] Tests added/updated
- [ ] Documentation updated
- [ ] No clippy warnings
- [ ] Code formatted with `cargo fmt`
- [ ] Commit messages are clear

## Reporting Issues

When reporting issues, please include:

1. Description of the issue
2. Steps to reproduce
3. Expected vs actual behavior
4. Rust version and OS
5. Relevant error messages or logs

## Security Issues

For security vulnerabilities, please contact the maintainers privately rather than opening a public issue.

## Code of Conduct

- Be respectful and inclusive
- Focus on constructive feedback
- Help others learn and grow

## License

By contributing, you agree that your contributions will be licensed under MIT OR Apache-2.0.
