# Home Trader Application

Home Trader is a Rust-based stock market trading simulator with a microservice architecture. It features a cross-platform GUI for visualization and a separate trading engine for processing market data and simulations. This project is an implementation based on the `.home-trader-spec.md` technical specification.

## Overview

- **Trading Engine**: Core service written in Rust, handles data processing, indicator calculation, and trading simulations via gRPC.
- **GUI Client**: Cross-platform desktop application (built with Dioxus) for real-time chart rendering, user interaction, and configuration.
- **Shared Library**: Common Rust code, including data models, shared between the engine and GUI.

## Project Structure

```
home-trader/
├── Cargo.toml                 # Workspace definition
├── README.md
├── docs/                      # Documentation (architecture, API, development guidelines)
│   ├── architecture.md
│   ├── api.md
│   └── development.md
├── engine/                    # Trading Engine (Rust binary crate)
│   ├── Cargo.toml
│   ├── build.rs               # Proto compilation
│   ├── src/                   # Engine source code
│   └── proto/                 # gRPC .proto definitions
│       └── trading.proto
├── gui/                       # GUI Client (Rust binary crate using Dioxus)
│   ├── Cargo.toml
│   ├── src/                   # GUI source code
│   └── assets/                # GUI assets (e.g., default configuration)
│       └── config/
│           └── default.json
├── shared/                    # Shared library (Rust lib crate)
│   ├── Cargo.toml
│   └── src/                   # Shared source code (models, utils)
└── tests/                     # Integration and data tests
    ├── integration/           # Integration tests
    └── data/                  # Sample data for testing (e.g., sample.csv)
```

## Getting Started

Refer to the `.home-trader-spec.md` (section 14) for detailed prerequisites and build instructions.

### Basic Build & Run (from within this `home-trader` directory)

1.  **Build Workspace**:
    ```bash
    cargo build --workspace
    ```

2.  **Run Tests**:
    ```bash
    cargo test --workspace
    ```

3.  **Run Trading Engine**:
    ```bash
    cargo run --bin engine
    ```

4.  **Run GUI Client** (in a separate terminal):
    ```bash
    cargo run --bin gui
    ```

## Development

See `docs/development.md` for development guidelines, code style, and Git workflow.

## Specification

The detailed technical specification for this project can be found in `.home-trader-spec.md`.

---

This README provides a brief overview. For more detailed information, please refer to the documents in the `docs/` directory and the main specification file.
