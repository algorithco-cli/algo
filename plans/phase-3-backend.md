# Phase 3 — Backend (Cloud + Teams)

> Exit: login→org→policy→daemon→audit→dashboard E2E, backup/restore tested.
> Stack: Rust axum tokio tonic/ConnectRPC, Postgres sqlx (compile-checked), Valkey (rate-limit), Zitadel (OIDC device flow SAML SCIM), OpenBao or SOPS+age, Garage S3, octocrab (P4). Start Postgres-only; ClickHouse/NATS only on measured need via ADR.

## P3-01 Proto backend API (first)

`Auth(device tokens)`, `Org/Team/Role`, `PolicyBundle{version,signed_bytes,sig}`, `PolicyDryRun`, `AuditIngest{redacted_event,decision,latency}` (no source field), `StatsQuery`. ConnectRPC service defs. Tag; Rust+TS clients compile.

## P3-02 Core verify helpers

Sig verify (detached, pinned key, version ordering, rollback protection) + redacted audit-record builder (fails closed if not redacted) in `core` (heavily tested), not ad hoc. Tampered/expired/rollback rejected; secret-in-payload fails; mutants on verify path. Human review.

## P3-03 Auth + orgs

OAuth device flow (`algo login` polls); orgs/teams/roles CRUD; every endpoint authed + rate-limited (Valkey); migrations versioned + reversible; no secrets in logs; testcontainers-real-Postgres integration. AC: CLI login E2E vs ephemeral stack; unauthed/over-limit rejected; up/down tested.

## P3-04 Policy sync + ingest

Publish → signed bundle → daemon polls/verifies/applies; `dry-run` replays vs redacted history; audit ingest opt-in redacted (drops source if present); stats aggregates; apalis/Postgres queue. AC: publish→apply within interval; tampered rejected client-side; secret-bearing audit dropped+logged; backup/restore tested.

## P3-05 Agent sync client (opt-in, local-first)

`algo login`, `algo policy pull/apply`, `algo log` upload gated by privacy (`local-only` disables net except Jev-if-configured). Offline → L0-L1 continue, cloud deferred. `algo doctor` sync health. AC: network-killed decisions meet L0/L1; egress capture has zero source bytes (proxy test); pause/uninstall unaffected.
