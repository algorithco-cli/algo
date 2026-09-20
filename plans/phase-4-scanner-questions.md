# Phase 4 — Scanner + Question Assistant

## Scanner (`core` pre-filter + `agent/scanner`, capability #4)

- Tree-sitter + rule pre-filter (secrets, missing-auth, injection) finds candidates; Jev judges batched/budgeted; surfaced post-edit, quiet unless finding.
- Findings → SQLite + (opt-in) backend/dashboard; `Finding` schema from P4-01 with redacted excerpt only.
- Never blocks on scanner timeout (decision already made; advisory + logged, off critical hook path).
- AC: precision/recall on labeled vuln-file set published; secret fixture triggers redacted-only finding; post-edit perf off critical path.

## Question assistant (`agent`, capability #5, Choice-only)

- On `agent.question`: free_form → always user/LLM, never Jev. Choice → Jev picks via project context + past choices (local, never overriding hard rules), confidence-gated (low → ask user).
- "Always allow this kind" → local rule/training signal only with consent; hard-deny override attempt rejected + tested.
- AC: Choice accuracy on labeled set; free-form-never-auto proven; override-rejection test.
