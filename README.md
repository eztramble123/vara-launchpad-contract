# Vara Contract Template Library

A production-ready library of smart contract templates for the [Vara Network](https://vara.network), built with [Sails-RS](https://github.com/gear-tech/sails).

## Overview

This library provides 8 battle-tested smart contract templates covering DeFi, DAO/Governance, and utility patterns. Each contract includes comprehensive documentation, test suites, and security considerations.

## Contracts

| Contract | Category | Description |
|----------|----------|-------------|
| [Access Control](./contracts/access-control/) | Utility | Role-based access control (RBAC) |
| [Escrow](./contracts/escrow/) | DeFi | Milestone-based escrow with dispute resolution |
| [Vesting](./contracts/vesting/) | DAO | Time-locked token distribution |
| [Crowdfunding](./contracts/crowdfunding/) | Utility | Campaign-based fundraising |
| [Voting](./contracts/voting/) | DAO | On-chain governance system |
| [Reputation](./contracts/reputation/) | DAO | Reputation scoring and badges |
| [Lending](./contracts/lending/) | DeFi | Collateralized lending protocol |
| [Launchpad](./contracts/launchpad/) | DeFi | Token launch platform |

## Quick Start

### Prerequisites

- Rust (stable toolchain)
- wasm32v1-none target: `rustup target add wasm32v1-none`

### Build All Contracts

```bash
cargo build --release
```

### Run Tests

```bash
cargo test --release
```

### Build Individual Contract

```bash
cargo build --release -p escrow
```

## Project Structure

```
vara-contracts-library/
├── contracts/
│   ├── access-control/    # RBAC contract
│   ├── escrow/            # Escrow contract
│   ├── vesting/           # Vesting contract
│   ├── crowdfunding/      # Crowdfunding contract
│   ├── voting/            # Governance contract
│   ├── reputation/        # Reputation system
│   ├── lending/           # Lending protocol
│   └── launchpad/         # Token launchpad
├── shared/                # Shared types and utilities
└── docs/                  # Documentation
```

## Documentation

- [Deployment Guide](./docs/deployment-guide.md)
- [Security Checklist](./docs/security-checklist.md)
- [Integration Patterns](./docs/integration-patterns.md)

## Technical Stack

- **Framework**: Sails-RS v0.9.1
- **Rust Edition**: 2021
- **Build Target**: wasm32v1-none
- **Testing**: gtest + tokio

## License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

## Contributing

Contributions are welcome! Please read our contributing guidelines before submitting PRs.
# vara-contract-templates
# vara-launchpad-contract
