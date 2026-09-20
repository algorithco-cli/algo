# algo-provider — DecisionProvider trait + mock (Jev OFF)

Trait `DecisionProvider { judge(&self, event: &RedactedEvent, qs: &[TypedQuestion]) -> Result<TypedAnswers, ProviderError> }` per `plans/phase-1-05-core-provider.md`.

Jev stays **OFF** in product code (only trait + mock) per waiver `docs/adr/0009-p0-gate-waiver.md` + `docs/redact-consent-readiness.md` — no `api.typesafe.ai` calls, no real data leaves, no embedded key.

`MockProvider` is deterministic from a fixture map for daemon/adapter tests + offline eval harness.
