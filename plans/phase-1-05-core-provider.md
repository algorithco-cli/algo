# P1-05 Core: provider (`P1-CORE-5`)

`core/crates/provider/src/{lib,trait,mock,jev}.rs`

## Trait

```rust
trait DecisionProvider {
  fn judge(&self, event: &RedactedEvent, qs: &[TypedQuestion])
    -> Result<TypedAnswers, ProviderError{Timeout,Auth,Net,Parse}>;
}
TypedQuestion = Bool | Choice(options) | Score(0..1)
```

Zero business logic — transport + serde only.

## MockProvider

Deterministic from fixture map. For daemon/adapter tests + offline eval harness.

## JevProvider

reqwest h2 pool, warm on daemon start, single batched request (all questions at once), timeout 700ms, 1 retry 200ms budget, typed errors.

[VERIFY]: REST spec, BYOK header auth, rate limits, zero-retention flag. If unverified → behind `feature="jev"` stub + `#[ignore]` test.

## Acceptance

Mock passes eval offline; wiremock test proves timeout→`ProviderError::Timeout` and caller maps to ask; no secrets in logs (asserted).
