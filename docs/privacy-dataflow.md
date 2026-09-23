# Privacy + dataflow — data residency + consent drafts (owner-confirmed 2026-09-20)

> **Owner facts recorded 2026-09-20** (public pages + owner confirmation):
> no AUP exists (legal index lists only DPA, MCA, Privacy Policy; `typesafe.ai/legal/aup`
> 404, MCA §2.3(l) dangling); subprocessors **AWS** (stores) / **Modal, Nebius,
> CoreWeave** (process, do not store) / **Slack, Google Workspace** (support) — **all
> USA** (`trust.typesafe.ai/subprocessors`); Privacy Policy "Services" — TypeSafe
> **will not train on Input** and **will not disclose Input except to service
> providers**; retention is **"as long as reasonably necessary"** with **no fixed SLA**;
> hosted in the **US**; ZDR is **enterprise-only via `privacy@typesafe.ai`**.
> **R4 confirmed 2026-09-23** (owner-researched; Privacy Policy "International
> Visitors" Nov 19, 2025 + DPA §6 Apr 24, 2026): US-only hosting, no region-selection
> option; EEA/UK transfers under EU SCCs / UK Addendum — reflected in the consent
> text below.
> Text here must match future `algo init` behavior and the consent below.
> No vendor claims without a measurement link.

## Modes — BYOK only, Jev off by default

| Mode | Behavior | Jev? |
|------|----------|------|
| `local-only` (**default**) | No network. No Jev call. Cloud sync off. | **Off** — no data leaves the machine |
| `redacted` (BYOK, explicit opt-in) | Secrets masked before anything leaves. **Only real-data mode.** Requires `ALGO_JEV_API_KEY` (BYOK, env-only) + consent. | On — **redacted** `canonical.redacted_payload` only, to **US** |
| `full` (BYOK, explicit opt-in) | Unredacted payloads allowed — requires second, clear consent + inspect step. | On — **unredacted** (only with `full` consent) |

**Defaults:** fresh `algo init` is `local-only`. BYOK is the **only** real-data mode.
No embedded key in binaries (MCA §2.4). Proxy mode through our servers stays
blocked until legal review (`docs/adr/0004-byok-vs-proxy.md`).

## Where your data goes (for the consent text)

- **Who:** TypeSafe AI, Inc. and its service providers (subprocessors above — all USA).
- **Where:** **US infrastructure** (Privacy Policy "International Visitors": Services are **hosted in the US**). If you use the Services from the EEA/UK or other non-US regions, you **transfer personal data to the US** for storage and processing.
- **How long:** **unspecified** — "as long as reasonably necessary" (Privacy Policy
  "Retention"; DPA Schedule I §8 "as long as necessary" per purpose + statute of
  limitations). No fixed retention period, no deletion SLA is published.
- **Training:** TypeSafe states it **will not train or fine-tune AI/ML models on your prompts or other Input** (Privacy Policy "Services") and **will not disclose Input to a third party other than our service providers** (same section). MCA §4.1/§4.3 separately grants a perpetual telemetry license on the other telemetry (hashes/stats/learnings), with no weight training without your prior consent (withholdable).
- **Zero-retention:** ZDR is **enterprise-only via `privacy@typesafe.ai`** — no ZDR on standard plans.

## Consent text draft (for `algo init` privacy prompt — BYOK + `redacted`)

> **How Jev works when you turn it on.** `local-only` (the default) never sends
> anything to Jev. If you choose `redacted` (BYOK), every command you approve for
> Jev is **redacted on your machine** and then sent to **TypeSafe AI's US
> infrastructure** (`api.typesafe.ai`) under your own Jev API key. You set the key
> as `ALGO_JEV_API_KEY` (BYOK, env-only, never embedded). TypeSafe says it **will
> not train on your Input** and **will not disclose Input except to its US service
> providers** listed above. It keeps your data **as long as reasonably necessary**
> (no fixed deletion date). **Jev requests are processed in the United States
> regardless of your location. If you are in the EEA or UK, TypeSafe transfers
> this data under Standard Contractual Clauses / the UK Addendum.** If you are
> outside the US, your data is **transferred to the US**. You can inspect exactly what would leave with `algo log --show-egress`.
> Switching back to `local-only` stops all sending.

> **Privacy notice draft (for `web/` / docs footers):** "When Jev is enabled (BYOK,
> `redacted` or `full`), your redacted (or with `full`, unredacted) commands are
> sent to TypeSafe AI's US infrastructure under your own key, processed by its US
> subprocessors (AWS / Modal / Nebius / CoreWeave), retained as long as reasonably
> necessary (no fixed SLA is published), and covered by TypeSafe's Privacy Policy
> statement that it will not train on Input and will not disclose Input except to
> service providers. Non-US users transfer data to the US (EEA/UK: under Standard
> Contractual Clauses / the UK Addendum). `local-only` sends
> nothing. ZDR is available only via enterprise `privacy@typesafe.ai`."

## Rules

- Redact-before-network, always (one code path for send and `--show-egress`).
- Inspect-what-would-send: `algo log --show-egress` shows the **exact** redacted outbound payload.
- Telemetry is opt-in and never contains source code.
- Datasets are redacted. No real secrets. No user code without consent.
- AUP does **not** exist (owner-confirmed 2026-09-20; `typesafe.ai/legal/aup` 404) — written confirmation requested, **not a blocker**. MCA §2.3(l) reference is dangling.
- Consent records `consent_id` + timestamp + scope (`redacted` vs `full`) + BYOK key holder; stored outside the dataset.

## Flow

```text
agent event -> adapter (parse) -> redact -> L0/L1/L2 (local, no network)
  -> [L3 Jev — only if mode != local-only AND BYOK key present AND consent is redacted/full:
       redacted only, hosted US, retention "as long as reasonably necessary",
       non-US -> US transfer, timeout -> ask]
  -> decision + reason -> local audit (SQLite ~/.algo/audit.db, redacted_command only)
  -> [opt-in backend sync: redacted only]
```
