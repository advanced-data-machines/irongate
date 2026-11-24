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

## Expanded acceptance checkpoint

[`mvp-lab-v1`](lab-acceptance.md) now freezes the engineering targets separately from current one-client capacity. The live suite expands to eleven cases with positive-control/ingress/egress spoof evidence and a dedicated IPv6 kernel guard exercised after crash. Schema-2 reports retain the target contents/hash and explicitly mark scale/accounting qualification `not_run`; seven-case schema-1 reports cannot pass the expanded validator.

The checkpoint has 124 passing Rust tests, 16 framework and 14 lab-contract regressions. Shared owner turns and descriptor validation have 100% ordinary-test line coverage. CRAP now analyzes 216 functions and flags only the existing CLI parser at 39.32; the earlier startup threshold finding is resolved, while guest bootstrap/serving coverage remains incomplete. See the [burndown](burndown.md) for product gates still pending.

## Local consent checkpoint

The [native consent lab](local-consent-portal.md) adds real browser form submission, typed owner requests, one-use scoped actions and a bounded local record writer. Its current development quality checkpoint passes **135 Rust tests**, **16 comparison regressions** and **17 host-only lab/portal evidence tests**. CRAP analyzes **251 functions**, with only the preexisting CLI parser above 30 at **39.32**. HTTP worker startup is still uncovered by ordinary unit-test LCOV (score 30); live guest scenarios are independent functional evidence and are not merged into coverage.

Schema-3 `local-consent-v1` reports require six portal cases plus all eleven packet/isolation cases. Browser namespace, rejection statuses, the ordered 16-record durable prefix and screenshot identities/digests must agree before the host accepts a run. Failed runs stay retained. The first complete development browser run is `67dffaba03ee6841751fe1591106d181` (dirty source); retain clean-commit reruns separately. All 17 scenarios passed, establishing Gate A for the fixed lab. This does not qualify RADIUS accounting, normal deployment or scale targets. Record synchronization promises only acknowledged rows, not crash-complete billing.

## HTTPS consent checkpoint

The [HTTPS consent feature](portal-https.md) adds explicit TLS configuration, bounded transport workers and scheme-specific Host/Origin checks while preserving static musl artifacts. Its development checks pass 139 Rust tests and 35 Python regressions. CRAP analyzes 279 functions, with only the preexisting CLI parser above 30 at 39.32; worker startup still has incomplete ordinary-test coverage. Live execution is not merged into LCOV.

The version-4 `local-https-v1` suite requires all 17 prior portal/packet cases plus independent browser/Python certificate rejection and TLS deadline/queue/malformed-record pressure evidence. Initial development run `bdc380b045883d2fd67d5f7d4435e4e7` established verified browser TLS and two occupied-worker deadlines, before the final queue-overflow checks were added; it is dirty development evidence. Retain final clean-commit runs separately with the full contract, three verified screenshots, quality identity, dependency graph, static ABI checks and exact footprint/memory observations. Production certificate automation, RADIUS and scale qualification remain pending.

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
