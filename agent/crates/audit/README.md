# algo-audit

SQLite WAL audit store `~/.algo/audit.db` via `AuditStore`.
Never raw secrets: caller must redact; store only `redacted_command`.

Usage: `AuditStore::open(path)?.init()?`, `insert(decision, fingerprint, shadow)`, `last()`, `counts()`, `list(limit)`.
Pragmas: WAL + synchronous NORMAL + busy_timeout 5000.
