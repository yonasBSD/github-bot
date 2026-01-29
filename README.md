# github-bot

[![Licenses](https://github.com/yonasBSD/github-bot/actions/workflows/licenses.yaml/badge.svg)](https://github.com/yonasBSD/github-bot/actions/workflows/licenses.yaml)
[![Linting](https://github.com/yonasBSD/github-bot/actions/workflows/lint.yaml/badge.svg)](https://github.com/yonasBSD/github-bot/actions/workflows/lint.yaml)
[![Testing](https://github.com/yonasBSD/github-bot/actions/workflows/test-with-coverage.yaml/badge.svg)](https://github.com/yonasBSD/github-bot/actions/workflows/test-with-coverage.yaml)
[![Security Audit](https://github.com/yonasBSD/github-bot/actions/workflows/security.yaml/badge.svg)](https://github.com/yonasBSD/github-bot/actions/workflows/security.yaml)
[![GitHub Release](https://img.shields.io/github/release/yonasBSD/github-bot.svg)](https://github.com/yonasBSD/github-bot/releases/latest)
[![License](https://img.shields.io/github/license/yonasBSD/github-bot.svg)](https://github.com/yonasBSD/github-bot/blob/main/LICENSE.md)

A powerful GitHub bot for your terminal that integrates GitHub client commands with an extensible plugin system.

## Features

- **Terminal-First Design**: Interact with GitHub directly from your command line
- **Workspace Architecture**: Organized as a Cargo workspace with separate CLI and library crates
- **Plugin System**: Extensible architecture supporting custom plugins
- **GitHub Integration**: Seamless integration with GitHub's API and features via the octocrab library
- **Git Operations**: Built-in Git functionality for repository management
- **Cross-Platform**: Built in Rust for performance and reliability across platforms
- **Comprehensive CI/CD**: Extensive GitHub Actions workflows for quality assurance

## Architecture

The project is organized as a Cargo workspace with two main components:

### CLI (`backpack/cli`)
The command-line interface that users interact with. It provides:
- Command parsing and execution
- User-facing commands (hello, maintain, merge, wip)
- Integration with the core library

### Library (`backpack/lib`)
The core functionality library that provides:
- GitHub API integration (`github/` module)
- Git operations (`git/` module)
- Plugin system (`plugins/` module)
- CLI utilities (`cli/` module)

This separation allows the core functionality to be reusable in other contexts while keeping the CLI focused on user interaction.

## Installation

### From Source

```bash
git clone https://github.com/yonasBSD/github-bot.git
cd github-bot
cargo build --release
```

The CLI binary will be available at `target/release/cli`.

### Using Cargo

```bash
cargo install --git https://github.com/yonasBSD/github-bot --bin cli
```

## Quick Start

```bash
# Run the bot
./target/release/cli

# Or if installed via cargo
cli

# View available commands
cli --help

# Run specific commands
cli hello
cli merge --help
```

## Configuration

The bot can be configured using environment variables or a configuration file. Create a `.env` file in the project root or set the following environment variables:

```env
GITHUB_TOKEN=your_github_token
GITHUB_USER=your_username
```

### Configuration File

Create a configuration file (e.g., `config.toml`) with your preferred settings:

```toml
[github]
token = "your_github_token"
user = "your_username"

[plugins]
enabled = ["plugin1", "plugin2"]
```

## Plugin Development

github-bot includes a plugin system located in `backpack/lib/src/plugins/`. The plugin architecture supports extensible functionality for GitHub operations.

### Plugin Structure

The plugin system is organized in the library crate (`backpack/lib`):

```
backpack/lib/src/plugins/
├── mod.rs       # Plugin trait and core functionality
├── color.rs     # Color plugin implementation
└── tests.rs     # Plugin tests
```

### Creating a Plugin

Plugins implement the core plugin trait defined in the library. Here's a basic example:

```rust
use github_bot_lib::plugins::Plugin;

pub struct MyCustomPlugin {
    // Plugin state
}

impl Plugin for MyCustomPlugin {
    fn name(&self) -> &str {
        "my_custom_plugin"
    }
    
    fn execute(&self, context: &PluginContext) -> Result<(), PluginError> {
        // Your plugin logic here
        Ok(())
    }
}
```

### Available Plugins

- **Color Plugin** (`color.rs`) - Provides colorized output functionality

To add your plugin:

1. Create a new file in `backpack/lib/src/plugins/`
2. Implement the `Plugin` trait
3. Register it in `backpack/lib/src/plugins/mod.rs`
4. Add tests in the plugin's test file

## Available Commands

The bot provides several commands for GitHub workflow automation:

### Core Commands

- **`hello`** - Welcome/introductory command
- **`maintain`** - Repository maintenance operations
- **`merge`** - Pull request merge operations and workflows
- **`wip`** - Work-in-progress management

For detailed help on each command:

```bash
github-bot <command> --help
```

## Development

### Prerequisites

- Rust 1.70 or later
- Cargo
- Git

### Building

This is a Cargo workspace project with multiple crates:

```bash
# Clone the repository
git clone https://github.com/yonasBSD/github-bot.git
cd github-bot

# Build all workspace members
cargo build

# Build in release mode
cargo build --release

# Build specific workspace member
cargo build -p cli

# Run tests for all workspace members
cargo test

# Run the CLI
cargo run -p cli

# Or after building
./target/release/cli
```

### Development Tools

This project uses a comprehensive set of development tools:

- **Formatting**: `cargo fmt` (configured via `rustfmt.toml`)
- **Linting**: `cargo clippy` (configured via `clippy.toml`)
- **Testing**: `cargo nextest` (configured via `.config/nextest.toml`)
- **Task Automation**: Multiple options available:
  - `just` (justfile)
  - `cargo-make` (Makefile.toml)
  - `task` (Taskfile.dist.yaml)
- **Git Hooks**: 
  - Lefthook (`.lefthook.toml`)
  - Pre-commit (`.pre-commit-config.yaml`)
- **Changelogs**: `git-cliff` (cliff.toml) or `cocogitto` (cog.toml)
- **Version Management**: `mise` (`.mise.toml`)
- **Security Scanning**: `trivy` (trivy.yaml)
- **License Checking**: `cargo-deny` (deny.toml)

```bash
# Format code
cargo fmt

# Run clippy
cargo clippy --all-targets --all-features

# Run tests with nextest
cargo nextest run

# Run all checks (using just)
just check

# Or using cargo-make
cargo make check

# Or using task
task check

# Install git hooks
lefthook install
# or
pre-commit install
```

## CI/CD

The project includes comprehensive GitHub Actions workflows for:

- **Code Quality**: Formatting and linting checks
- **Testing**: Automated tests with code coverage
- **Security**: Security audits and vulnerability scanning
- **Packaging**: Release builds and artifact generation
- **Cross-Platform**: Multi-platform build verification

## Project Structure

This is a Cargo workspace project with the following structure:

```
github-bot/
├── .cargo/              # Cargo configuration
├── .config/             # Tool configurations (nextest, etc.)
├── .github/             # GitHub Actions workflows
│   └── workflows/       # CI/CD pipeline definitions
├── backpack/            # Main application workspace
│   ├── cli/            # Command-line interface
│   │   └── src/
│   │       └── commands/  # Command implementations
│   │           ├── hello/      # Hello command
│   │           ├── maintain/   # Maintenance commands
│   │           ├── merge/      # Merge operations
│   │           └── wip/        # Work-in-progress commands
│   └── lib/            # Core library
│       ├── src/
│       │   ├── cli/           # CLI utilities
│       │   ├── git/           # Git operations
│       │   ├── github/        # GitHub API integration
│       │   └── plugins/       # Plugin system
│       └── tests/             # Integration tests
├── manifests/           # Kubernetes/deployment manifests
├── packaging/           # Package build configurations
│   └── nfpm/           # NFPM (packaging tool) specs
├── xtask/              # Build automation tasks
├── Cargo.toml          # Workspace manifest
└── README.md           # This file
```

## Contributing

Contributions are welcome! Please follow these guidelines:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run tests and linting (`cargo test && cargo fmt && cargo clippy`)
5. Commit your changes (`git commit -m 'Add amazing feature'`)
6. Push to the branch (`git push origin feature/amazing-feature`)
7. Open a Pull Request

Please ensure your code follows the project's coding standards and includes appropriate tests.

## Security

For security concerns, please review [SECURITY.md](SECURITY.md) and report vulnerabilities responsibly.

## License

This project is licensed under the MIT License - see the [LICENSE.md](LICENSE.md) file for details.

## Acknowledgments

- Built with [Rust](https://www.rust-lang.org/)
- GitHub API integration powered by [octocrab](https://github.com/XAMPPRocky/octocrab)
- Comprehensive CI/CD workflows
- Inspired by the need for efficient terminal-based GitHub workflows

## Support

- **Issues**: [GitHub Issues](https://github.com/yonasBSD/github-bot/issues)
- **Discussions**: [GitHub Discussions](https://github.com/yonasBSD/github-bot/discussions)

## Roadmap

See [TODO.md](TODO.md) for planned features and improvements.

## Related Projects

- [rust-ci-github-actions-workflow](https://github.com/yonasBSD/rust-ci-github-actions-workflow) - Template used for CI/CD setup

---

Made with ❤️ and Rust
