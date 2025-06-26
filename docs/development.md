# Development Guidelines

This document outlines guidelines for developing the Home Trader application.

- **Code Style**: Follow standard Rust conventions (rustfmt).
- **Testing**: Unit tests for individual modules, integration tests for service interactions.
- **Git Workflow**: Use feature branches, follow conventional commit messages (e.g., `feat:`, `fix:`, `docs:`, `test:`).
- **Line Limits**: Adhere to specified line limits per file (e.g., 450 lines for GUI files).
- **Error Handling**: Use `anyhow` for application-level errors, `thiserror` for library-specific error types.
- **Logging**: Use the `tracing` crate for structured logging.
