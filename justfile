# List available commands.
default:
    @just --list

# Build the workspace.
build:
    cargo build --workspace

# Format Rust sources.
fmt:
    cargo fmt --all

# Run the test suite.
test:
    cargo test --workspace

# Check formatting, lint, and test.
check:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace

# Run the operator CLI, forwarding arguments unchanged.
[positional-arguments]
ig *args:
    cargo run --quiet -p irongate-cli -- "$@"
