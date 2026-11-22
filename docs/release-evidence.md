# Release evidence

Launch reports must be reproducible, retained and tied to the exact source being released. A successful build, a low CRAP score or an isolated replay result cannot establish gateway readiness on its own. This document defines the evidence to collect as the implementation reaches each acceptance gate.

## Current quality checkpoint

The first coverage-backed quality check used `cargo-crap 0.5.0` and `cargo-llvm-cov 0.9.1` on the **uncommitted `feature/lease-core` working tree**. It is a development checkpoint, not an immutable release baseline. All 92 Rust tests passed during coverage collection.

The report analyzed 158 functions across 39 source files, with all 39 files matched to LCOV. One function exceeded the tool's default threshold of 30. Test/example/benchmark directories and nested unit-test files are excluded from source analysis; uncovered production functions remain included under the default pessimistic missing-coverage policy.

| Function | Source | CRAP | Reported line coverage |
| --- | --- | --- | --- |
| CLI argument parser | [`args::parse`](../crates/irongate-cli/src/args.rs) | 39.32 | 94.0% |
| DHCP option validation | [`options::validate`](../crates/irongate-proto/src/dhcp/options.rs) | 29.03 | 96.6% |
| Configuration validation | [`validate::check`](../crates/irongate-config/src/validate.rs) | 28.40 | 92.0% |
| Highest lease-core score | [`Table::request`](../crates/irongate-core/src/lease/requests.rs) | 14.14 | 91.1% |

CRAP is a per-function complexity/coverage metric, not one project score or a performance measurement. The CLI parser is the first refactoring candidate: high coverage does not remove its branching complexity. Keep meaningful validation and protocol checks; improve responsibilities and tests rather than deleting checks or hiding difficult functions to improve a number.

Run `just crap` to regenerate fresh workspace coverage and the complete JSON/Markdown reports in `target/quality/`. It requires installed coverage/analysis tools and cached Cargo dependencies, with no new runtime crates. The console prints only the top ten functions; use the complete JSON report for totals and release decisions. The recipe currently reports findings without failing on the threshold, and is separate from `just check`.

## Session foundation checkpoint

The [session feature](session-core.md) extends the development checkpoint to 113 passing Rust tests and 190 analyzed functions across 46 matched source/LCOV files. The highest new session CRAP score is 12.0, with no new functions over 30; the CLI parser finding remains open. The session replay measured equal setup/replay heap totals over 100000 extra admission/cleanup cycles. Its worker is simulated, so these results establish neither live enforcement nor gateway performance. Keep the original checkpoint above as historical evidence rather than silently replacing its scope.

## First live packet checkpoint

The [Linux packet lab](linux-packet-lab.md) passed seven live scenarios in fresh NIC-free QEMU overlays, with independent BusyBox/TCP/DNS client evidence and a persistent nftables crash guard exercised with kernel routing enabled and neighbors fixed. It records per-run source/binary/image/tool/topology identity, exact case outcomes and cleanup status. This is a one-client functional checkpoint, not Gate A or measured capacity.

The accompanying checkpoint passed 122 Rust tests, 16 comparison-framework regressions and 10 host-only lab-contract tests. CRAP now analyzes 211 functions across 54 matched files. The CLI parser remains at 39.32; the new live `io::run` entry scores 207.02 with about 13% ordinary-test coverage. Guest execution is not merged into LCOV. That startup/coverage finding is open, not waived; retain the distinction between scenario evidence and coverage-backed quality evidence before launch.

## Release bundle contract

Before publishing a release candidate, retain the following together:

| Evidence | Required contents |
| --- | --- |
| Identity | Release version, exact source commit, clean/dirty status, UTC run time, lockfile hash, compiler/target and analysis-tool versions |
| Correctness | Formatting/lint/test logs and explicit command exit status; supported configuration/protocol scope and known failures |
| Quality | Full CRAP JSON and readable report, LCOV input, exact scope/exclusions, threshold and comparison with the previous accepted release |
| Allocation | Raw baseline/replay totals, workload/reset/warmup definition, iteration count, tool/build profile and memory-check exit status |
| C/Rust comparison | Canonical versioned JSON plus readable report, pinned C revision, fixture/adapter identity, expected differences and matched/unmatched workload status |
| Live qualification | VM/client interoperability, portal admission/revocation and isolation, backend/accounting failure tests, then declared target-hardware load/latency/soak results |
| Review | Supported deployment profile, acceptance decisions, unresolved limitations and explicitly reviewed exceptions |

Release evidence must describe a clean, frozen commit. A dirty-tree run is useful development evidence but must not be attributed to its HEAD commit as if the changes were included there. Keep raw reports immutable after acceptance and use a new run identity for reruns. Separate run directories prevent accidental replacement or merging of evidence from different builds.

`target/` is ignored, disposable build storage; it is not the release archive. Before launch, add CI/release packaging that uploads the full evidence bundle alongside the matching release and preserves prior accepted bundles for comparisons. That archival automation is **not implemented yet**. Keep this checkpoint and the collection contract in Git; retain generated raw release artifacts in the release/CI artifact store, without subscriber data, secrets or private configuration.

## Qualification policy to implement before launch

- Collect fresh coverage from the exact tested source. Treat missing/mismatched coverage, failed commands and incomplete artifacts as unavailable evidence, never a passing zero score.
- Compare quality using compatible tool versions, source scope, exclusions and coverage settings. Review added complexity, lost coverage and new functions explicitly; tool changes require a documented new baseline.
- Resolve functions above CRAP 30 before launch or record a specific reviewed exception with rationale and follow-up. The existing CLI parser finding is open; this document does not waive it. Review near-threshold functions without treating the threshold as a complete correctness test.
- Keep correctness, access enforcement, accounting integrity, allocation and performance as separate gates. No average or composite score can offset a failed requirement.
- Publish only claims supported by the retained workload and environment. Offline allocation/helper comparisons do not establish live throughput, interoperability, RADIUS health or production stability.

The [performance/stability criteria](performance-stability.md), [comparison framework](comparison-framework.md), [lease evidence](lease-core.md) and [MVP acceptance gates](mvp-plan.md) define the supporting scopes. Numerical live-performance targets must still be ratified and measured before they become launch claims.
