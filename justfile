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

# Coverage-backed complexity report; requires cargo-llvm-cov and cargo-crap.
crap:
    mkdir -p target/quality
    cargo llvm-cov --workspace --lcov --output-path target/quality/lcov.info --offline
    cargo crap --workspace --lcov target/quality/lcov.info --exclude '**/tests/**' --exclude '**/tests.rs' --format json --output target/quality/crap.json
    cargo crap --workspace --lcov target/quality/lcov.info --exclude '**/tests/**' --exclude '**/tests.rs' --format markdown --output target/quality/crap.md
    cargo crap --workspace --lcov target/quality/lcov.info --exclude '**/tests/**' --exclude '**/tests.rs' --top 10

# Measure replay heap allocations using system Valgrind (optional).
protocol-allocations:
    cargo build -p irongate-proto --example protocol_replay --release
    python3 scripts/check-protocol-allocations.py "${CARGO_TARGET_DIR:-target}/release/examples/protocol_replay"

# Demonstrate lease ownership and complete DHCP replies without networking.
lease-demo:
    cargo run -p irongate-proto --example lease_demo --offline

# Check steady-state allocations across 100000 complete lease lifecycles.
lease-allocations:
    cargo build -p irongate-proto --example lease_replay --release --offline
    python3 scripts/check-protocol-allocations.py "${CARGO_TARGET_DIR:-target}/release/examples/lease_replay"

# Demonstrate offline consent, policy acknowledgements and revocation.
session-demo:
    cargo run -p irongate-core --example session_demo --offline

# Check heap totals across 100000 admission/cleanup lifecycles.
session-allocations:
    cargo build -p irongate-core --example session_replay --release --offline
    python3 scripts/check-protocol-allocations.py "${CARGO_TARGET_DIR:-target}/release/examples/session_replay"

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

# Provision a separate guest image; temporary QEMU userspace NAT downloads packages.
vm-lab-prepare:
    python3 -m tools.vm.lab.prepare

# Inspect the NIC-free lab command without preparing or booting a VM.
vm-lab-command:
    python3 -m tools.vm.lab.run --print

# Test VM report/isolation contracts without starting a VM or touching networking.
vm-lab-check:
    python3 -m unittest tools.vm.lab.test_host tools.vm.lab.test_portal

# Prepare Chromium only inside a separate lab image with temporary userspace NAT.
vm-portal-prepare:
    python3 -m tools.vm.lab.prepare_portal

# Exercise the real local consent page from a browser inside the client namespace.
vm-portal-test:
    cargo build -p irongated --release --target x86_64-unknown-linux-musl --offline
    python3 -m tools.vm.lab.run --portal

# Build the static lab adapter; then boot a fresh guest-only packet scenario.
vm-lab-test:
    cargo build -p irongated --release --target x86_64-unknown-linux-musl --offline
    python3 -m tools.vm.lab.run

# Render the two-page performance brief using an installed Chrome/Chromium browser.
brochure:
    python3 scripts/render-brochure.py

# Run the operator CLI, forwarding arguments unchanged.
[positional-arguments]
ig *args:
    cargo run --quiet -p irongate-cli -- "$@"

# Prepare guest-only OpenSSL/NSS certificate tooling in a separate image.
vm-https-prepare:
    python3 -m tools.vm.lab.prepare_portal --https

# Run HTTPS consent and TLS failure checks in a fresh NIC-free guest.
vm-https-test:
    cargo build -p irongated --release --target x86_64-unknown-linux-musl --offline
    python3 -m tools.vm.lab.run --https
