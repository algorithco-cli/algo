# algo-provider — DecisionProvider trait + mock + feature-gated Jev client

Trait `DecisionProvider { judge(&self, event: &ToolBefore, qs: &[TypedQuestion]) -> Result<TypedAnswers, ProviderError> }`.

Jev stays **OFF by default** (only trait + mock) — no `api.typesafe.ai` calls, no real data leaves, no embedded key. The real client lives behind `--features jev` (`src/jev.rs`: pooled blocking HTTP, one batched request, 700ms timeout, 1 retry on 429/529, strict spec-shape parsing where every deviation maps to a typed error the caller turns into `ask`). The daemon keeps `MockProvider` until the redact + consent + shadow gates clear; key comes from `ALGO_JEV_API_KEY` env-only and never appears in logs, errors, or `Debug`.

`MockProvider` is deterministic from a fixture map for daemon/adapter tests + offline eval harness.
