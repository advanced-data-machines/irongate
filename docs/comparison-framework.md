# Reusable comparison and regression framework

The offline comparison is a maintained development tool, not a one-off benchmark. New protocol/state behavior should bring shared cases, independent correctness checks, declared differences and bounded-memory evidence when equivalent operations exist. It must serve the [portal/RADIUS MVP plan](mvp-plan.md), rather than delaying the demonstration with unrelated microbenchmarks.

## Routine commands

```sh
just check
just comparison-check
just coova-compare --list-suites
just coova-compare --checks-only
just coova-compare --suite dhcp-options --checks-only
just coova-compare
```

`comparison-check` runs 16 Python standard-library regression tests without compilers, Valgrind, a Coova checkout or host-network access. `coova-compare` additionally builds pinned original C and optimized Rust probes. Correctness always runs with optimized C, sanitized C and Rust; `--checks-only` skips timing/Valgrind. `--suite` is repeatable and defaults to all registered suites. Unknown suite names fail. No added runtime dependencies, service startup or network I/O are involved. Tool/sanitizer requirements are in [the Coova comparison](coova-comparison.md).

Current suites are `dhcp-options` v1 (18 cases) and `dhcp-offers` v1 (four cases). They preserve all initial comparison observations. Every run writes versioned JSON and readable Markdown under the selected `--output`; use distinct output directories for concurrent runs. Starting a new run replaces old success with `in_progress`, and exceptions produce an explicit failed/incomplete report. Do not treat a stale report from a failed command as a pass. JSON is the canonical status artifact.

Report schema 2 uses a `suites` array with suite name/version/description, cases, raw observations and input hashes, benchmark workloads, memory workloads and measurement status. `complete` concerns the selected executed checks, not gateway readiness; skipped timing/memory remain explicit. Suite versions identify comparison semantics separately from the report schema or binary version. Schema-1 reports remain historical artifacts and are not accepted as performance baselines by this runner.

## Small extension contract

| Module | Responsibility |
| --- | --- |
| `framework.py` | `Case`, `Workload`, `Suite`, generic process execution, empty/duplicate case rejection |
| `suites.py`, `corpus.py`, `wire.py` | Explicit suite registry, domain cases, expected differences and independent wire oracle |
| `source.py`, `build.py`, `adapter.[ch]` | Immutable original C extraction, notices/hashes, scoped builds and intercepted side effects |
| `measure.py`, `benchmarks.py` | Alternating C/Rust sample order, raw batch results, separate Valgrind comparisons |
| `baseline.py` | Same-implementation comparisons and an optional caller-selected regression gate |
| `evidence.py`, `render.py`, `run.py` | Provenance, report lifecycle, generic rendering and CLI wiring |

A suite declares a stable name, version, description, cases, comparison callback, benchmark workloads and optional memory workloads. A case supplies a unique name, probe mode, bounded input bytes, a small numeric selector and explicit expectations. The generic executor runs all three probes, then asks the domain comparator to verify statuses, values/effects and the independent oracle. An unexpected difference or sanitizer failure is a failure; known differences require visible classifications. Verification remains active under `python -O`.

The current probe convention is `PROBE MODE SELECTOR ITERATIONS`, bounded bytes on stdin and structured JSON on stdout. Adapter setup, fixture decoding, formatting and one warmup stay outside timed loops. A future session suite may consume a bounded event trace rather than packet bytes, but must define its state reset, iteration boundary, returned effects and exhaustion semantics. No FFI or plugin-loading machinery is required. C extraction/build selection remains explicit and must be extended for each actual new source boundary; arbitrary future Coova subsystems are not automatically supported by the DHCP fixture contexts.

To add a comparison:

1. Pick the matching C/Rust operation and define identical inputs, initial state, allowed effects, units and reset behavior. Record which work differs and whether a full daemon or helper is being tested.
2. Add focused fixture builders and expected outcomes: normal, malformed, exact/full/over capacity, duplicate/stale/reordered events as applicable. Include ingress network identity where relevant. Keep credentials and subscriber data out of source control.
3. Add only necessary original C functions and narrowly scoped fixture adapters. Preserve original bodies, source identity and notices; intercept network/disk effects rather than triggering them.
4. Register a suite with a domain comparator and independent oracle. Add negative oracle tests so wrong identities, missing effects or checksums cannot accidentally count as equivalent.
5. Add benchmark/memory workloads only once they exercise meaningful matching operations. `cross_implementation_comparable` defaults to false; retain a concrete caveat until work, output and instrumentation are matched. Do not print a C/Rust speedup ratio for unmatched paths.
6. Run framework tests, affected suite correctness/sanitizers, memory checks and repeated timings. Bump the suite version when expectations, timed work, reset policy or interpretation changes; explain and review any new expected difference.

The next suites should follow the MVP dependencies: DHCP ACK/NAK and lease state, generation-safe admission/revocation, then RADIUS response validation and accounting counters. DNS and forwarding comparisons follow their actual implementation boundaries. VM client-to-service tests complement this harness; the harness alone does not establish captive access enforcement.

## Saved performance baselines

Save an immutable successful schema-2 report before changing an implementation:

```sh
just coova-compare
cp target/coova-compare/report.json target/coova-baseline.json
just coova-compare --baseline target/coova-baseline.json
```

This compares C with its earlier C run and Rust with its earlier Rust run, not C with Rust. It retains raw samples and reports median change percentages without imposing an invented release threshold. To request a gate, pass an explicit percentage, for example `--max-regression-percent 10`; that number is illustrative, not IronGate's adopted budget. A requested gate returns nonzero on a regression, missing baseline workload or incompatible evidence. Baseline and output report must be different files.

Compatibility checks cover recorded CPU/model/OS/affinity, compilers, relevant build flags/profiles, C source and fixture context, probe adapters, suite version, input/mode/selector, iteration count and at least three samples in both runs. Changed production Rust implementation is expected; changed benchmark adapters or fixture meaning require a new baseline. Incomparable evidence is reported, not silently passed. Existing reports with failed or incomplete result status cannot establish a baseline. These safeguards do not control ambient load, thermal state, frequency scaling or statistical significance: repeat anomalous runs on a controlled machine before making performance claims.

Both current workloads remain marked unmatched across languages. The framework currently enforces correctness, sanitizer success and zero additional measured replay allocation; performance gating is opt-in and applies only to compatible same-implementation observations. Target-hardware packet rate, tail latency and accounting-lag gates will be separate end-to-end suites once a live gateway exists.
