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

### OS Dependencies for GUI (Linux)

When building the Dioxus-based GUI on Linux, you'll likely need the following development libraries. These were the versions confirmed to work during setup:

- **GTK3**: `sudo apt-get install libgtk-3-dev` (or equivalent for your distribution)
- **WebKit2GTK**: `sudo apt-get install libwebkit2gtk-4.1-dev` (specifically version 4.1, or equivalent for your distribution)
- **GLib 2.0**: `sudo apt-get install libglib2.0-dev` (or equivalent for your distribution)
- **XDO**: `sudo apt-get install libxdo-dev` (or equivalent for your distribution)
- **pkg-config**: `sudo apt-get install pkg-config` (often installed as part of a build-essential package)
- **C compiler**: `sudo apt-get install build-essential` (provides gcc and other necessary tools)

These dependencies are required by the underlying webview renderer and other libraries that Dioxus uses for desktop applications. Ensure these are installed before attempting to build or run the GUI client.
