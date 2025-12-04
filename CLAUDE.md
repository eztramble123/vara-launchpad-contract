# Vara Launchpad Contract - Project Context

## Overview
This is a standalone launchpad contract for the Vara Network, extracted from the vara-contracts-library.

## Current Status
- **Build Issue**: The WASM build fails with `#[panic_handler] function required, but not found`
- The issue stems from gear-wasm-builder generating projects with `edition = "2024"` hardcoded

## Technical Stack
- **Framework**: Sails-RS v0.9.1
- **Gear Core**: gstd/gtest/gclient v1.9.1
- **Rust**: 1.88.0
- **Build Target**: wasm32v1-none (should be installed)

## Key Files
- `contracts/launchpad/app/src/lib.rs` - Main business logic (LaunchpadService)
- `contracts/launchpad/tests/gtest.rs` - Integration tests
- `contracts/launchpad/build.rs` - Build script that calls sails_rs::build_wasm()

## Known Issues Being Debugged
1. gear-wasm-builder 1.9.1 generates WASM projects with `edition = "2024"` which may cause panic_handler issues
2. The panic_handler should come from gstd via cfg(target_arch = "wasm32") + feature = "panic-handler"

## Build Commands
```bash
cargo build --release -p launchpad
cargo test --release -p launchpad
```

## Reference
This project was duplicated from vara-contracts-library which contains 8 contracts total.
See ../vara-contracts-library for the full template library.
