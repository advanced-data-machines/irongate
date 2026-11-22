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

# Measure replay heap allocations using system Valgrind (optional).
protocol-allocations:
    cargo build -p irongate-proto --example protocol_replay --release
    python3 scripts/check-protocol-allocations.py "${CARGO_TARGET_DIR:-target}/release/examples/protocol_replay"

# Check comparison contracts without requiring Coova, a C compiler or Valgrind.
comparison-check:
    python3 -m unittest discover -s tools/coova_compare -p 'test_*.py'

# Compare selected suites offline; requires cc, clang, and optionally Valgrind.
[positional-arguments]
coova-compare *args: comparison-check
    python3 tools/coova_compare/run.py "$@"

# Fetch the pinned Debian base and verify SHA-512; does not boot anything.
vm-fetch:
    python3 tools/vm/image.py

# Prepare a private writable overlay and offline cloud-init seed; no boot/network.
vm-prepare:
    python3 tools/vm/prepare.py

# Print the exact first-boot command without executing it.
vm-command:
    bash tools/vm/boot.sh --print

# Boot the learning VM with KVM and a serial console, with no NIC.
vm-boot:
    bash tools/vm/boot.sh

# Render the two-page performance brief using an installed Chrome/Chromium browser.
brochure:
    python3 scripts/render-brochure.py

# Run the operator CLI, forwarding arguments unchanged.
[positional-arguments]
ig *args:
    cargo run --quiet -p irongate-cli -- "$@"
