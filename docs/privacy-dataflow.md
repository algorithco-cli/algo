# Privacy + dataflow (stub — Phase 0)

Text here must match future `algo init` behavior. No vendor claims without a measurement link.

## Modes

| Mode | Behavior |
|------|----------|
| `local-only` | No network except explicit Jev path if configured. Cloud sync off. |
| `redacted` (default) | Secrets masked before anything leaves the machine. |
| `full` (explicit opt-in) | Unredacted payloads allowed; requires clear consent + inspect step. |

## Rules

- Redact-before-network, always.
- Inspect-what-would-send: `algo log --show-egress` shows the exact outbound payload.
- Telemetry is opt-in and never contains source code.
- Datasets are redacted. No real secrets. No user code without consent.

## Flow

```text
agent event -> adapter (parse) -> redact -> L0/L1/L2 (local)
  -> [L3 Jev: redacted only, timeout -> ask]
  -> decision + reason -> local audit (SQLite ~/.algo/audit.db)
  -> [opt-in backend sync: redacted only]
```
