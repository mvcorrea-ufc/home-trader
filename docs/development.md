# Development Guidelines

This document outlines guidelines for developing the Home Trader application.

- **Code Style**: Follow standard Rust conventions (rustfmt).
- **Testing**: Unit tests for individual modules, integration tests for service interactions.
- **Git Workflow**: Use feature branches, follow conventional commit messages (e.g., `feat:`, `fix:`, `docs:`, `test:`).
- **Line Limits**: Adhere to specified line limits per file (e.g., 450 lines for GUI files).
- **Error Handling**: Use `anyhow` for application-level errors, `thiserror` for library-specific error types.
- **Logging**: Use the `tracing` crate for structured logging.

## GUI Development

The GUI is built using the Dioxus framework. Key libraries include:

- **`dioxus`**: The core Dioxus library for building user interfaces with a React-like component model.
- **`dioxus-desktop`**: Provides the desktop renderer for Dioxus applications.
- **`fuzzy-matcher`**: Used for implementing fuzzy search functionality, for example in the command palette.
