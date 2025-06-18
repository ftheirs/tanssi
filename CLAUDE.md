# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

Tanssi is a permissionless appchain infrastructure protocol designed for swift and effortless deployment of application-specific blockchains (container chains). It's built on Substrate/Polkadot SDK and provides orchestrator chains that manage collator assignment and container chain registration.

## Development Commands

### Building
```bash
# Standard release build
cargo build --release

# Build with fast runtime (for testing - shorter sessions)
cargo build --features=fast-runtime --release

# Build TypeScript packages
pnpm build

# Generate TypeScript API interfaces (required before running tests)
cd typescript-api && pnpm create-local-interfaces && cd ..
```

### Testing
```bash
# Run all Rust unit tests
cargo test --features=fast-runtime --release

# Run a specific Rust test
cargo test --features=fast-runtime --release -- test_name --exact

# Run TypeScript integration tests (requires Node 22.12.0)
cd test
pnpm moonwall test dev_tanssi      # Manual seal orchestrator tests
pnpm moonwall test zombie_tanssi   # Zombienet tests with container chains

# Run specific test suites
pnpm moonwall test dev_tanssi --grep "test description"
```

### Linting & Formatting
```bash
# Rust formatting (requires nightly)
cargo +nightly fmt

# Rust linting
cargo clippy --all-targets --all-features -- -D warnings

# TypeScript linting and formatting
pnpm lint
pnpm fmt
```

## Architecture

### Core Components
- **Orchestrator Chains**: Dancebox (parachain), Dancelight/Starlight (relay chains)
- **Container Chains**: Templates for Simple and Frontier (EVM-compatible) chains
- **Key Pallets**:
  - `pallets/registrar`: Container chain registration
  - `pallets/collator-assignment`: Dynamic collator management
  - `pallets/author-noting`: Block authorship tracking
  - `pallets/services-payment`: Payment system for container chain services

### Directory Structure
- `chains/`: Chain implementations (orchestrator and container chains)
- `pallets/`: Custom Substrate pallets
- `client/`: Client-side components and consensus
- `primitives/`: Shared types and traits
- `test/`: TypeScript integration tests using Moonwall framework
- `typescript-api/`: Generated TypeScript interfaces

### Key Concepts
1. **Container Chains**: Custom blockchains that can be deployed on Tanssi
2. **Collator Assignment**: Automatic assignment of block producers to container chains
3. **Data Preservation**: Service for maintaining historical chain data
4. **XCM Integration**: Cross-chain messaging between container chains and orchestrator

### Testing Infrastructure
- Moonwall framework for integration testing
- Zombienet for multi-node network simulations
- Manual seal mode for deterministic testing
- Fast runtime feature for shorter session times in tests

### Important Configuration Files
- `moonwall.config.json`: Test suite configurations
- `clippy.toml`: Rust linting rules
- `biome.jsonc`: TypeScript/JavaScript linting and formatting
- Chain spec files in `test/moonwall/configs/*/specs/`