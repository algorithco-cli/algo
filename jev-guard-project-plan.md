# algorithco guard: Master Project Plan

> **Audience:** AI coding agents (and humans) building this product across multiple repositories.
> **Status:** Working plan. Product name DECIDED: **algorithco guard**, CLI **`algo`** (`algo init|doctor|status|why|log|enforce|pause|login|policy`). Proto package `algorithco_guard.v0`, Rust crates `algo-*`, home dir `~/.algo/`, socket `~/.algo/algo.sock`, DB `~/.algo/audit.db`. Items marked **[VERIFY]** must be checked against official documentation before implementation. Items marked **[DECISION]** are open and need an ADR (Architecture Decision Record) before work starts.
> **Scope of this document:** product, architecture, per-repository specs, contracts, quality gates, and build order. Infrastructure/DevOps (VPS provisioning, IaC, observability stack) is out of scope for now and will get its own plan.

---

## 1. What we are building

`algorithco guard` is an **intelligent control layer for CLI coding agents** (Claude Code, Codex CLI, OpenCode). It does not replace those agents. It attaches to them through their hook/plugin/MCP mechanisms and makes them safer, faster to work with, and more trustworthy.

It uses **Jev**, a probabilistic decision model from TypeSafe AI. Jev does not generate text. Given a state and a set of typed questions, it returns typed answers (Boolean, Choice, Score) with probabilities. It is reported by its vendor to be much faster and cheaper than LLMs for this kind of work. **[VERIFY]** These are vendor claims; we measure them ourselves in Phase 0.

### Core capabilities (in priority order)

1. **Safe auto-approve.** Before an agent runs a command or edits a file, decide `allow`, `deny`, or `ask` with a confidence value. Confident-safe actions pass without interrupting the user. Uncertain actions go to the user. Dangerous actions are blocked.
2. **Verifier.** When the agent says "done", check whether it really is (tests run? change matches the task?). Can send the agent back to work.
3. **Loop controller.** Detect repeated failures or runaway loops and stop, retry differently, or ask the user.
4. **Security scanner.** After edits, check changed files for common security problems (exposed secrets, missing auth checks, injection patterns) using cheap pre-filters plus Jev judgments.
5. **Agent-question assistant (later).** When the agent asks a multiple-choice question, Jev can pick from the options using project context and past user choices. Free-form questions are never answered by Jev; they go to the user or an LLM.
6. **Team layer (later).** Shared policies, audit, dashboards, GitHub PR/CI triage.

### Who it is for

Developers using CLI agents daily who are tired of approval prompts and of agents claiming success falsely, plus teams that need auditability and policy control.

---

## 2. Non-negotiable principles

These override convenience. Every repo must respect them.

1. **Fail-safe.** Any error, timeout, crash, missing model, parse failure, or unreachable service results in `ask`. **Never** in `allow`.
2. **Deterministic rules outrank models.** Hard deny rules run before any model and cannot be overridden by Jev or by learned preferences.
3. **Performance is a feature.** The hook sits in the critical path of every agent action. Latency budgets (section 4) are enforced in CI. A change that breaks a budget does not merge.
4. **Local-first.** The local tool must be useful with no cloud account. Cloud features are additive.
5. **Privacy by default.** Secrets are redacted before anything leaves the machine. Users can see exactly what is sent. A local-only mode exists. Telemetry is opt-in and never contains source code.
6. **Explainable decisions.** Every decision has a short human-readable reason and is written to a local audit log.
7. **Reversible install.** Installation modifies agent config files only after explicit consent, keeps backups, and `uninstall` restores everything.
8. **Provider abstraction.** Jev is accessed behind an interface so it can be swapped or supplemented (for example with a small local model or another LLM) without touching the rest of the system.
9. **No invented APIs.** If an agent's hook format or Jev's API is not confirmed from official docs, mark it **[VERIFY]**, do not guess.

---

## 3. Organization and repository layout

The project is a **multi-repo organization**, not a monorepo. Each repo has one clear responsibility, its own CI, its own release cycle, and its own `AGENTS.md`.

| Repo | Language | Responsibility |
|---|---|---|
| `proto` | Protobuf (+ buf) | Single source of truth for cross-repo contracts: canonical events, decisions, API schemas. |
| `core` | Rust | Shared libraries: canonical types, policy engine, shell analysis, redaction, Jev provider interface. No I/O side effects beyond what its crates declare. |
| `agent` | Rust | Everything that runs on the user's machine: hook client, daemon, CLI, TUI, agent adapters, verifier, loop controller, scanner, local model runtime. |
| `backend` | Rust | Cloud API: auth, orgs, policy distribution, audit ingestion, stats, GitHub App, optional Jev proxy, training-data pipeline (opt-in). |
| `dashboard` | TypeScript (React) | Web app for teams: decision history, security findings, policies, stats. |
| `web` | TypeScript (Astro) | Marketing site, documentation, install script hosting. |
| `eval` | Python | Datasets, labeling tools, evaluation harness, threshold tuning, distillation training, model export. |
| `docs` | Markdown | Org-level: this plan, ADRs, security model, threat model, contribution rules. |
| `infra` | (later) | Deployment and infrastructure. Placeholder; planned separately. |

### Dependency direction (no cycles allowed)

```
proto  ──►  core  ──►  agent
   │          │
   │          └──────►  backend
   ├────────────────►  dashboard   (generated TS client)
   └────────────────►  eval        (schemas for datasets)

eval ──(exports model artifact)──►  agent
```

### Rules for multi-repo work

- **`proto` is the contract.** Changing a contract means a PR to `proto` first, with `buf breaking` checks passing, then version bump, then consumers update. No repo hand-writes types that duplicate a proto message.
- **Versioning:** semantic versions, tagged releases. Consumers pin exact versions of `proto` and `core`.
- **Distribution of `core`:** as private Rust crates via git tags or a private cargo registry. **[DECISION]** Choose one; default to git tags until the registry exists.
- **Generated code:** consumers generate code from a tagged `proto` release at build time. Do not commit hand-edited generated code.
- **Cross-repo changes** are done in order: `proto` → `core` → `agent`/`backend` → `dashboard`. Each step is a separate PR linked to a tracking issue in `docs`.
- **Each repo contains an `AGENTS.md`** describing build/test commands, conventions, and what requires human review.

---

## 4. System architecture

```
 Claude Code      Codex CLI       OpenCode
     │                │               │
     ▼                ▼               ▼
 [adapter]        [adapter]       [adapter]        (in `agent`, one crate each)
     └───────────────┬───────────────┘
                     ▼
        Thin hook client  ──socket──►  Daemon (long-running)
                                          │
                     ┌────────────────────┴───────────────────┐
                     │        Decision pipeline (L0 → L4)     │
                     └────────────────────┬───────────────────┘
                                          ▼
                       allow | deny | ask  + reason + confidence
                                          ▼
                    Adapter renders the answer in the agent's format
                                          ▼
                                 Local audit log (SQLite)

  Optional cloud (opt-in):  daemon ◄──► backend ◄──► dashboard
```

### 4.1 The five-level decision pipeline

Speed comes from the fact that **most requests never reach Jev**. The traffic shares below are **hypotheses to be measured in Phase 0/1**, not facts.

| Level | What | Target latency | Notes |
|---|---|---|---|
| **L0** | Hard rules (deny/allow lists) evaluated on the **syntax tree** of the command, not raw text | microseconds | Highest authority. Cannot be overridden. |
| **L1** | Cache keyed by normalized action fingerprint | microseconds | Normalization removes irrelevant variation (paths, hashes, timestamps). |
| **L2** | Small local classifier (CPU), trained from Jev outputs and user confirmations | 1 to 5 ms | Distillation. Grows more useful over time. See caveats below. |
| **L3** | Jev evaluation (remote) | network-bound | Only for new or uncertain cases. Multiple questions per single request. |
| **L4** | Ask the user | human | Also the universal fallback. |

**Latency budget (enforced in CI benchmarks):**

| Path | p50 | p99 |
|---|---|---|
| L0/L1 (local) | < 3 ms | < 10 ms |
| L2 (local model) | < 10 ms | < 25 ms |
| L3 (Jev) | < 250 ms | < 800 ms |

If real measurements show these are unreachable, adjust through an ADR, not silently.

**Additional performance techniques:**

- **Speculative evaluation:** pre-evaluate likely next actions to warm the cache. **[DECISION]** Only build after basic pipeline is measured.
- **Batching questions:** ask Jev many questions about the same state in one request.
- **Warm connections:** persistent HTTP/2 connection to Jev, opened when the daemon starts.
- **Daemon:** avoids cold-start cost per hook call. The hook client is a tiny binary whose only job is to talk to the daemon.
- **Degraded mode:** if the network or Jev is unavailable, L0 to L2 keep working; everything else becomes `ask`.

**Caveats on L2 distillation (must be resolved before building it):**

- **[VERIFY]** TypeSafe's terms of service: are we allowed to use Jev outputs to train another model?
- User confirmations as training data require explicit user consent and must never include unredacted secrets or source code.
- L2 must be evaluated against the same eval set as Jev. It ships only if its false-allow rate is no worse than the agreed threshold.

### 4.2 Decision outputs

Each decision contains: action (`allow`/`deny`/`ask`), reason (one line, human-readable), confidence, source level (`rule`/`cache`/`local_model`/`jev`/`fallback`), and latency. The `algo why` command prints the reason for the most recent decision.

---

## 5. Repository specifications

### 5.1 `proto`

**Purpose:** contracts shared by all repos.

**Contains:**
- Canonical event types: `tool.before`, `tool.after`, `agent.stop`, `agent.question`, with agent identity, session id, working directory, tool kind (`shell`, `edit`, `write`, `read`, `net`, `other`), and payload.
- Decision type (see 4.2).
- Hook client ↔ daemon protocol.
- Daemon ↔ backend API (auth, policy sync, audit ingestion, stats).
- Dataset record schema used by `eval`.

**Tooling:** buf (lint, breaking-change detection, codegen), Protobuf, ConnectRPC/gRPC.

**Definition of done:** `buf lint` and `buf breaking` pass in CI; tagged releases generate Rust and TypeScript packages; every message has documentation comments.

### 5.2 `core`

**Purpose:** shared, heavily tested libraries. This is where correctness matters most.

**Crates (suggested):**
- `types`: canonical types generated from `proto`, plus helpers.
- `shell-analysis`: parses shell commands with **tree-sitter-bash** into a syntax tree and extracts facts (commands, flags, redirections, pipes, subshells, obfuscation patterns such as variable expansion and encoded payloads). Rules match on structure, never on raw strings only.
- `policy`: rule engine. **[DECISION]** CEL (`cel-rust`, check maturity) versus a small custom declarative DSL. Rules are compiled once at load. Hard deny list ships built in.
- `redact`: secret detection and masking (gitleaks-style rules ported to Rust using `regex` and `aho-corasick`), applied before any data leaves the machine.
- `provider`: the `DecisionProvider` interface (Jev implementation, mock implementation, later a local-model and generic-LLM implementation). Typed questions in, typed answers with probabilities out, explicit timeout and error types.
- `fingerprint`: action normalization for the cache.

**Quality requirements:**
- Fuzzing (`cargo-fuzz`) on `shell-analysis` and `policy`.
- Property tests (`proptest`) for normalization and redaction.
- Mutation testing (`cargo-mutants`) on the hard-rule paths.
- A public regression suite of known-dangerous and known-safe commands, including obfuscated variants.
- `clippy`, `cargo-deny`, `cargo-audit` clean.

**Human review required** for any change to hard rules, the default deny list, or auto-approve thresholds logic.

### 5.3 `agent` (everything on the user's machine)

**Purpose:** the product the user installs.

**Components:**

| Component | Description | Tech |
|---|---|---|
| Hook client | Tiny binary invoked by the agent's hook. Sends the event to the daemon, prints the decision. Target ~1 ms startup. If the daemon is unreachable, it tries to start it once, otherwise returns `ask`. | Rust |
| Daemon | Long-running process holding the pipeline, cache, warm Jev connection, local model, audit writer. | Rust, tokio, Unix socket / Windows named pipe |
| Adapters | One crate per agent: `parse(raw) → CanonicalEvent` and `render(Decision) → agent format`. Thin and separately versioned. | Rust |
| CLI (`algo`) | `algo init`, `algo uninstall`, `algo doctor`, `algo status`, `algo why`, `algo pause`, `algo resume`, `algo login`, `algo log`, `algo policy`, `algo enforce`. Home dir `~/.algo/`, socket `~/.algo/algo.sock`, DB `~/.algo/audit.db`. | clap |
| TUI | Live decision feed, stats, policy editor. | ratatui |
| Jev client | HTTP/2, pooling, retries with budget, timeouts. **[VERIFY]** Official API and any Go/Rust SDK; otherwise build a small typed client from the official API spec. | hyper/reqwest |
| Local store | Audit log, cache persistence, user preferences. | SQLite (WAL) via rusqlite |
| Local model runtime (L2) | Loads the classifier artifact exported by `eval`. **[DECISION]** `ort` (ONNX Runtime) versus `candle`. | Rust |
| Verifier | Runs on the stop event. Checks that tests were run and that changes match the task. Can send the agent back to work. | Rust |
| Loop controller | Detects repeated identical failures and escalates. | Rust |
| Scanner | Tree-sitter plus rule pre-filter finds candidates; Jev judges them; results are surfaced after edits. | Rust |
| MCP server | For agents where hooks are limited. Uses the official Rust MCP SDK. **[VERIFY]** | rmcp |

**Agent integration reality (all **[VERIFY]** against current official docs before building each adapter):**
- **Claude Code:** hooks in settings (pre-tool-use can allow/deny/ask; stop hook can push the agent back). Start here.
- **Codex CLI:** config-based approval and sandbox settings, MCP, notify. Interception may be more limited than Claude Code. Design the adapter to degrade gracefully.
- **OpenCode:** plugin system and config. Confirm which events can block an action.

**`algo init` behavior:**
1. Detect installed agents.
2. Show an exact list of files that will change.
3. Ask consent per agent.
4. Back up existing config, then **add** hooks without removing existing ones.
5. Ask the privacy mode (see section 7).
6. Run `algo doctor` to verify.

`algo uninstall` restores from backup and removes the daemon. `algo pause` disables all interception instantly.

**Distribution:** single static binary (musl on Linux), install script, Homebrew tap, later winget/scoop and apt/rpm. Signed releases, SBOM, self-update with signature verification.

### 5.4 `backend`

**Purpose:** cloud services for teams and for optional features. Not required for local use.

**Stack:** Rust, axum, tokio, tonic/ConnectRPC, PostgreSQL with sqlx (compile-time checked queries), Valkey (cache/rate limit), Zitadel (self-hosted auth: OIDC, device flow, SAML, SCIM), OpenBao or SOPS+age (secrets), Garage (S3-compatible storage), octocrab (GitHub App). ClickHouse for high-volume decision analytics and NATS JetStream **only when measurements show the need**; start with Postgres.

**Responsibilities:**
- **Auth:** OAuth device flow so `algo login` works from the terminal.
- **Orgs, teams, roles.**
- **Policy distribution:** signed policy bundles; the agent verifies signatures before applying.
- **Audit and stats ingestion:** opt-in, redacted, no source code.
- **Optional Jev proxy mode:** **[DECISION]** see section 9.
- **Background jobs:** Postgres-backed queue (apalis or a small custom engine).
- **GitHub App:** PR risk scoring, CI failure triage (later phase).
- **Training data pipeline:** opt-in, consented, redacted, feeds `eval` (later phase; subject to the terms check in 4.1).

**Requirements:** migrations versioned and reversible; every endpoint authenticated and rate limited; integration tests with real Postgres via testcontainers; no secrets in logs.

### 5.5 `dashboard`

**Purpose:** team-facing web app.

**Stack:** React + Vite (static SPA build), TanStack Router/Query/Table, Tailwind, shadcn/ui, uPlot for large time series, Server-Sent Events for the live decision feed. API client is generated from `proto`.

**Views:** decision history with explanations, blocked/asked breakdown, security findings, policy editor with dry-run against history, per-user and per-project stats, time and cost savings.

**Requirements:** no Node.js runtime in production (static assets only); accessibility basics; Playwright end-to-end tests.

### 5.6 `web`

**Purpose:** landing page, documentation, install instructions.

**Stack:** Astro + Starlight. Static output. Documents the exact data-flow and privacy model in plain language.

### 5.7 `eval`

**Purpose:** the evidence that the product is safe and fast enough. **This is the most important repo in Phase 0.**

**Contains (Python: pandas, Jupyter, scikit-learn/PyTorch as needed):**
- **Datasets:** hand-labeled agent actions (`safe`, `dangerous`, `ambiguous`), 200 to 300 at first, growing. Versioned. Includes obfuscated and adversarial variants.
- **Question design:** experiments to find the Jev question phrasings that work best.
- **Harness:** runs any `DecisionProvider` (Jev, local model, baseline rules) over datasets and reports metrics.
- **Metrics:** **false-allow rate** (most critical), false-ask rate, false-deny rate, calibration of probabilities, latency p50/p95/p99, cost.
- **Threshold tuning:** produces the allow/deny confidence thresholds per profile (`strict`, `balanced`, `fast`).
- **Distillation pipeline:** trains the L2 classifier, exports ONNX, publishes a versioned model artifact consumed by `agent`.
- **CI gate:** results are published so that `core` and `agent` CI can fail on regressions.

**Data handling:** datasets are redacted; no real secrets; no user code without consent.

### 5.8 `docs`

ADRs, threat model, security model, privacy/data-flow description, contribution and review rules, this plan, roadmap, and a decision log.

---

## 6. Cross-cutting quality gates

Every repo's CI enforces its relevant subset:

| Gate | Where |
|---|---|
| Format, lint, type checks (clippy, Biome, mypy/ruff as appropriate) | all |
| Unit and integration tests | all |
| Fuzz and property tests | `core`, `agent` |
| Mutation tests on hard-rule paths | `core` |
| Latency benchmarks with hard budgets (criterion, hyperfine) | `core`, `agent` |
| **Eval gate:** false-allow must not exceed the agreed threshold | `core`, `agent`, `eval` |
| `buf lint` / `buf breaking` | `proto` |
| Dependency audit (cargo-deny, cargo-audit, cargo-vet, npm audit) | all |
| Agent end-to-end: real Claude Code/Codex/OpenCode driven headlessly against scripted scenarios | `agent` |
| Reproducible builds, signed release artifacts, SBOM | `agent`, `backend` |

**Human review is mandatory** (enforce via CODEOWNERS) for: the hard deny list, auto-approve threshold logic, redaction rules, signature verification, install/uninstall file modifications, and anything touching authentication.

---

## 7. User experience requirements

UX is half the product. A fast tool that feels risky will be uninstalled.

1. **Near-zero setup.** `algo init` is one command and finishes in about 30 seconds with sensible defaults.
2. **Shadow mode first.** For the first days, only observe and show "I would have auto-approved these". The user turns on enforcement when they trust it.
3. **Every decision is explainable.** `algo why` gives a one-line reason.
4. **Three profiles:** `strict`, `balanced`, `fast`. Advanced options hidden by default.
5. **Learning from the user.** If the user says "always allow this kind of action", record it as a local rule or training signal (never overriding hard deny rules).
6. **Status visibility.** A small indicator or command showing how many actions were auto-approved, asked, or blocked.
7. **One-step stop.** `algo pause` and `algo uninstall` always work, even if the daemon is broken.
8. **Transparent privacy.** Three modes: `local-only` (no network except Jev if configured), `redacted` (default), `full` (explicit opt-in). The user can inspect exactly what would be sent.
9. **Quiet by default.** No noisy output in the agent's UI unless something needs attention.

---

## 8. Build order

Each phase has an **exit gate**. Do not start the next phase until the gate passes.

### Phase 0: Contracts and evidence (first)
- `docs`: repo setup, ADR process, threat model draft.
- `proto`: first version of canonical event and decision types.
- `eval`: collect and label the first 200 to 300 actions; build the harness; get Jev API access; measure real accuracy and latency from target regions.
- **Exit gate:** measured false-allow and latency for Jev on the dataset. If Jev is not accurate or fast enough for a use case, redesign that use case before writing product code.

### Phase 1: Local MVP (no cloud)
- `core`: `types`, `shell-analysis`, `policy` (with hard deny list), `redact`, `provider` (Jev + mock), `fingerprint`.
- `agent`: hook client, daemon, Claude Code adapter (shell tool only), SQLite audit, `init`/`uninstall`/`doctor`/`pause`/`why`, **shadow mode**.
- **Exit gate:** benchmarks meet the L0/L1 budget; eval gate passes; install/uninstall verified on macOS and Linux; several real users run shadow mode for at least a few days.

### Phase 2: Enforcement and second capability
- Enforcement mode with thresholds from `eval`.
- Verifier (stop hook) and loop controller.
- Edit/write tool coverage.
- `web`: landing page and docs.
- **Exit gate:** measured reduction in approval prompts with false-allow within the threshold; no fail-open paths found in review and fuzzing.

### Phase 3: Cloud and teams
- `backend`: auth (device flow), orgs, signed policy sync, opt-in audit ingestion.
- `dashboard`: history, stats, policy editor.
- `eval` + `agent`: L2 local model (after the terms check).
- **Exit gate:** end-to-end team flow works; backup and restore tested; L2 meets the same eval threshold as Jev on its share of traffic.

### Phase 4: Expansion
- Codex and OpenCode adapters (each preceded by an **[VERIFY]** spike on interception capabilities).
- Security scanner.
- Agent-question assistant (Choice questions only).
- ratatui TUI, GitHub App (PR and CI triage), SSO/SCIM.
- Windows support hardening.

---

## 9. Open decisions and assumptions to verify

| # | Item | Why it matters |
|---|---|---|
| 1 | **Jev access mode [DECISION]:** users bring their own API key (better privacy, cheaper for us) versus routing through our server (single billing, shared cache and learning, but code passes through us). Design the `provider` interface to support both. | Determines privacy policy, backend scope, and cost model. |
| 2 | **Jev API and SDK [VERIFY]:** official REST spec, auth, rate limits, zero-data-retention options, availability of an on-prem/enterprise option. | Everything in L3 depends on it. |
| 3 | **Terms of use for training on Jev outputs [VERIFY].** | Gates the L2 distillation strategy. |
| 4 | **Hook/plugin capabilities of each agent [VERIFY].** | Determines what is possible per adapter. |
| 5 | Policy language: CEL vs custom DSL **[DECISION]**. | Core design. |
| 6 | Local model runtime: `ort` vs `candle` **[DECISION]**. | L2 performance and binary size. |
| 7 | License for each repo (for example permissive for `core`/`agent`) **[DECISION]**. | Trust and adoption for a security tool. |
| 8 | Payments provider (an external Merchant of Record or local options; legal check needed) **[DECISION]**. | Cannot be self-hosted; decide early. |
| 9 | Windows priority **[DECISION]**. | Named pipes, install paths, testing matrix. |
| 10 | Product name and branding — DECIDED: product `algorithco guard`, CLI `algo`, proto `algorithco_guard.v0`, crates `algo-*`, home `~/.algo/`. | Repos and package names depend on it. |

---

## 10. Working agreement for AI agents

1. **Read this plan and the repo's `AGENTS.md` first.** If they conflict, stop and ask; do not silently pick one.
2. **Do not invent external APIs, hook formats, or library capabilities.** Verify from official docs. If you cannot verify, leave a clearly marked `[VERIFY]` note and build behind an interface.
3. **Contracts first.** Never duplicate or hand-edit types that belong to `proto`.
4. **Write the ADR before the code** for any **[DECISION]** item.
5. **Fail-safe always.** When unsure, the behavior is `ask`. Add a test for every new failure path proving it does not allow.
6. **No secrets in code, logs, fixtures, or datasets.** Redact by default.
7. **Measure, do not assume.** Performance and accuracy claims need a benchmark or eval result in the PR.
8. **Small, reviewable PRs** with conventional commit messages, tests included, and docs updated.
9. **Security-critical paths need human sign-off** (see section 6). Do not merge them on agent approval alone.
10. **Leave the system uninstallable.** Any change that touches installation must have tests proving `uninstall` restores the original state.
11. **Flag uncertainty.** If a requirement is ambiguous, state the assumption in the PR description and ask.

---

## 11. Glossary

- **Hook:** a mechanism by which an agent calls external code at defined points (for example before running a tool).
- **Canonical event:** our agent-independent representation of something an agent is doing.
- **Adapter:** thin translator between one agent's format and canonical events/decisions.
- **Shadow mode:** observe and report decisions without enforcing them.
- **False-allow:** the system allowed something dangerous. The most important metric to minimize.
- **Distillation:** training a small local model to imitate a larger or remote model's decisions.
- **ADR:** Architecture Decision Record, a short document recording a decision and its reasoning.
