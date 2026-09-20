# 90 — Cross-cutting: Gates, UX, Decisions, Agreement

## 1. Quality gates (§6, CI merge-blockers per repo)

| Gate | Where | Rule |
|---|---|---|
| fmt/lint/type | all | clippy `-D warnings`, Biome, ruff+mypy — fail on warn |
| unit+integration | all | cargo test, vitest/playwright, pytest |
| fuzz+property | core,agent | cargo-fuzz shell/fs/policy/redact + adapter parse; proptest fingerprint/redact/thresholds; nightly 1h + PR 5min smoke |
| mutation hard paths | core | cargo-mutants deny+threshold+sig-verify; survivor = fail |
| latency | core,agent | criterion+hyperfine; L0/L1 <3/<10ms, L2 <10/<25ms, L3 <250/<800ms; over-budget = no merge or ADR |
| eval false-allow | core,agent,eval | harness JSON vs per-profile threshold; regression = fail |
| buf lint/breaking | proto | every PR; tags generate Rust+TS |
| dep audit | all | cargo-deny/audit/vet, npm audit |
| agent E2E headless | agent | Claude/Codex/OpenCode scripted incl. daemon-down/timeout/corrupt-model |
| reproducible+signed+SBOM | agent,backend | auditable/SLSA provenance, cosign/minisign verify, CycloneDX |
| human review CODEOWNERS | — | deny list, thresholds, redaction, sig-verify, `algo init`/`algo uninstall`, auth — agent approval never sufficient |

## 2. UX checklist (§7, every user-facing PR)

1. ~30s `algo init`: detect → diff → per-agent consent → backup additive → privacy prompt → `algo doctor`. Fresh-VM timing.
2. Shadow first: observe + "would-have N" digest; explicit `algo enforce on`.
3. Explainable: `algo why` = action+reason+confidence+source+latency; dashboard links same.
4. Profiles strict/balanced/fast visible; advanced hidden; threshold source in `algo status`.
5. Learning: always-allow → local rule/signal; hard-deny override rejected + tested.
6. Status counts + savings; SSE/TUI share query.
7. One-step stop: `algo pause`+`algo uninstall` daemon-broken.
8. Privacy local-only/redacted(default)/full(opt-in); `algo log --show-egress` inspector; telemetry opt-in never code.
9. Quiet unless attention; verifier/scanner batched post-action.
10. Colors: Variant 1 DECIDED — single source `design-tokens.md`; no hard-coded hex outside vars; allow/ask/deny tokens exclusive to decisions.

## 3. Open decisions §9 (ADR before code)

| # | Title | Blocks | Default |
|---|---|---|---|
| D-01 | Jev BYOK vs proxy | P3 scope/privacy/cost | provider supports both; ship BYOK first |
| D-02 | Jev API+SDK | all L3 | no default until verified (links+dates) |
| D-03 | ToS training | P3 L2 | L2 blocked until written clearance |
| D-04 | Hook caps/agent | P2 stop/edit, P4 | per-agent spike docs first |
| D-05 | CEL vs DSL | P2 policy | spike both, bias CEL if mature |
| D-06 | ort vs candle | P3-08 | benchmark both |
| D-07 | License/repo | public release | permissive core/agent proposal, legal sign-off |
| D-08 | Payments MoR | monetization | external MoR, early decide; reserve entitlement field in P3-01 |
| D-09 | Windows priority | P4-08 | macOS+Linux first, harden P4 unless pulled forward |
| D-10 | Name/branding — DECIDED | packages/web | product `algorithco guard`, CLI `algo`, proto `algorithco_guard.v0`, crates `algo-*`, home `~/.algo/` |

Each ADR: `docs/adr/NNNN-title.md` Status/Context/Options/Decision/Consequences/Verification. Branch `adr/D0X-*`, CODEOWNERS on `docs/adr/`, CI blocks D-* impl without merged ADR.

## 4. Working agreement §10 (enforcement)

1. Read plan + AGENTS.md; conflict → stop+ask. PR template checkbox.
2. No invented APIs: [VERIFY] link+date, behind trait; `grep VERIFY` fails if reaching main unlinked.
3. Contracts first: buf breaking + no proto-mirroring structs; cross-repo PRs link proto tag.
4. ADR before code for [DECISION].
5. Fail-safe: every I/O/timeout/parse path `proves_ask_on_*` test; kill allow-on-error mutants.
6. No secrets: gitleaks pre-commit + CI scan code/logs/fixtures/datasets.
7. Measure: PR needs benchmark or eval link + artifacts.
8. Small PRs: conventional commits, one repo per PR, docs updated, linked tracking issue.
9. Human sign-off security paths; branch protection.
10. Leave uninstallable: install PRs include `algo init→algo doctor→algo uninstall→diff` E2E.
11. Flag uncertainty: PR `Assumptions:` section; ambiguous → ask, not silent choice.
