# MVP burndown checkpoint

Status follows implemented acceptance evidence, not a percentage of checklist lines. There is no tracked, weighted effort history sufficient for a reliable percentage or delivery date. The normal deployment mode is still a scaffold; Gate A has passed in the fixed one-client isolated lab; gates B/C remain pending.

| Increment | Current status | Remaining to advance |
| --- | --- | --- |
| M0 Demo contract | Engineering lab target frozen as `mvp-lab-v1`; topology, units, limits and evidence boundaries recorded | Validate the targets as relevant subsystems become runnable; record any revised profile explicitly |
| M1 Lease/session core | Implemented, tested and integrated in `release/0.1.x` | Broader fuzz/interoperability and production recovery remain qualification work |
| M2 Linux packet lab | One-client functional path proven; expanded spoof/IPv6 evidence and startup refactor integrated into `release/0.1.x` | Load/capacity measurements, broader failure/fuzz coverage and production startup/recovery |
| M3 Local consent portal | Gate A demonstrated in QEMU: native browser/API consent, actual transit, logout/expiry and bounded synced records | Normal deployment wiring, recovery/rotation and broader qualification remain pending |
| M3b HTTPS consent | Integrated into `release/0.1.x` at `d119adf` | Production certificates, reload/renewal and broad compatibility remain pending |
| M4 RADIUS | Login slice integrated into `release/0.1.x` at `84abe82`: real backend, explicit configuration, typed attempt, enforced timeout and health observations | Bounded accounting integrated into `release/0.1.x` at `880a27d`; operational diagnostics/profile integration and Gate B acceptance remain |
| M4b External/cloud portal | Configuration contract exists; runtime pending | Secure handoff/callback, garden and tenant/device/network isolation |
| M5 MVP qualification | Pending | Two real network/VLAN profiles, repeatable mixed-provider scenarios, declared load/churn/soak and outage evidence; Gate C |

`release/0.1.x` now includes the expanded acceptance and consent portal features through `5695c36`, preserving their history by fast-forward. Gate A passed on that clean commit in run `b9b9896b713cbfb0cc8093a5ea59961a`; the retained report includes all 17 scenarios and three validated browser PNGs. This does not imply the larger scale/accounting targets passed. The [versioned contract](lab-acceptance.md) separates one-client functional evidence from the 64-session/two-network qualification target.

The local-consent feature builds on the acceptance checkpoint and has **135 passing Rust tests**, **16 comparison-framework regressions**, **17 host-only lab/portal regressions** and **17 live VM scenarios**. The coverage-backed CRAP report has **251 analyzed functions**; only CLI parsing remains above 30 at **39.32**. HTTP worker startup remains uncovered in ordinary unit-test LCOV (score 30), despite independent live browser evidence.

See the [local consent implementation](local-consent-portal.md) for bounds, action security, record health and acknowledged-prefix crash semantics. The initial successful development walkthrough is `67dffaba03ee6841751fe1591106d181`; it records a dirty working tree and must not be attributed to the parent commit as released code. Fresh clean-commit reports are retained under their own run IDs.

Next: operational/demo integration after review/integration of the accounting checkpoint against the [lab-alpha checklist](lab-alpha.md). The plan also defines a proposed static-musl Linux qualification matrix; broad distro support is not established yet. Production startup, journal recovery/rotation, normal TOML runtime wiring, two-VLAN/external hosting and load/accounting qualification remain explicit work. Keep CLI parser cleanup focused. Raw reports remain local development artifacts until release archival automation exists.

The HTTPS checkpoint adds four Rust regressions (**139 total**), two host TLS-evidence regressions (**19 lab + 16 comparison**) and a **19-case HTTPS VM suite**, while retaining the separate 17-case HTTP fixture. CRAP analyzes **279 functions**, with only the existing CLI parser above 30. See [HTTPS implementation and limits](portal-https.md). This was the prior quality checkpoint; the provider/authentication slice below supersedes it. Gate B is still pending.

The agreed [lab-alpha finish line](lab-alpha.md) has three increments: authentication, accounting, then a repeatable evaluated demonstration. Authentication currently passes 149 Rust tests, 20 lab/evidence tests, 16 comparison regressions and the 20-case RADIUS VM suite. CRAP analyzes 309 functions; only CLI parsing exceeds 30. See [RADIUS scope and evidence](radius-login.md). The full two-VLAN/cloud/scale MVP remains a later gate.

Integration checkpoint: annotated tag `checkpoint/radius-login` targets the tested runtime commit `84abe82`. Clean run `fe2936ad0b8c640dbe0039fcd6db23b0` passed all 20 RADIUS scenarios, with matching source/binary/lockfile and quality-bundle identity. This is an authentication checkpoint, not a completed lab alpha.

Accounting development checkpoint: `feature/radius-accounting` implements the second alpha increment, with an 18-case schema-6 VM suite and separate passive accounting status. Development run `122c603d251af060f6bb05cce7532238` passed all cases, matching independent directional workload/backend totals and outage/recovery. It is dirty evidence and predates the final status/module refinements. [Accounting bounds and crash limits](radius-accounting.md) remain explicit; final clean-source evidence is recorded in the handoff/release evidence. Runtime CLI/demo integration is the one remaining alpha increment, not the full-MVP qualification gate.

Clean accounting runtime `880a27d` passed run `b6be6a364080963b722ead5252d60630` (18 cases), plus separate authentication regression `56a0366f5358d5f20e178289e2775130` (20 cases), both dirty=false. The matching quality bundle records 161 Rust tests, 38 Python checks and 342 analyzed functions, with only the existing CLI parser above 30. The first two alpha increments are integrated into `release/0.1.x`; only operational/repeatable-demo integration remains. Accounting and its evidence documentation were merged by fast-forward; no new tag or push was performed.
