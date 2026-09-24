# ADR-0014: Release automation (tag → registries via GitHub Secrets/OIDC)

## Status

draft (does not authorize real publish while repo private per ADR-0009; needs CODEOWNERS + legal sign-off before first tag)

## Context

ADR-0013 made manifests publish-ready (dry-run). Next step: GitHub auto-publish
so setting secrets + pushing a tag publishes crates.io / PyPI / npm with no
manual uploads. Existing CI pattern: always-run gates on PR/push to `main`
(`ci.yml`, `rust.yml`, `eval.yml`, `web.yml`), secrets scanning (`secrets.yml`).

## Decision

- Three workflows (`.github/workflows/publish-{crates,pypi,npm}.yml`): dry-run on
  every PR/push to `main` (no secrets); real publish only on tag `release/*` or
  manual `workflow_dispatch` with `confirm=publish`, in GitHub Environment
  `release` (required reviewers = human sign-off).
- Secrets: `CARGO_REGISTRY_TOKEN` (crates.io publish token), `NPM_TOKEN`
  (npm Automation token for `@algorithco` scope). PyPI prefers Trusted Publisher
  (OIDC, no secret); `PYPI_API_TOKEN` is fallback only.
- Crates publish in topological order (types → … → tui; backend separately);
  preflight fails real publish on `LicenseRef-TBD` unless `PUBLISH_ALLOW_TBD=1`,
  and surfaces the `algo-types` proto-vendoring gap (`build.rs` references
  `../../../proto` outside the tarball).
- Only `packages/contracts` publishes to npm (`provenance`, `access public`);
  `dashboard`/`web` stay `private:true` and are asserted private in CI.
- Artifacts + attestations uploaded on every release run (dist tarballs, SBOM
  where produced).

## Alternatives

- **Single unified release workflow**: rejected — matches repo convention of
  per-domain workflows (`rust.yml`/`eval.yml`/`web.yml`) and keeps CODEOWNERS
  review granular.
- **Publish on every main push**: rejected — registries are immutable; tags +
  `release` environment gate prevent accidents.
- **Long-lived tokens everywhere**: rejected for PyPI — Trusted Publisher (OIDC)
  removes a stored secret; token kept as fallback only.

## Consequences

### Positive

- Set 2 secrets (+ 1 optional) + PyPI OIDC once → `git tag release/v0.1.0`
  publishes all three registries with provenance/attestations.
- Dry-runs on every PR catch metadata regressions before tags.

### Negative

- First real publish still blocked on ADR-0001 license + public repo + npm scope
  reservation + PyPI project creation + `algo-types` proto-vendoring fix.

## Verification

- Workflow YAML parses with the same `yaml.safe_load_all` gate as `ci.yml`
  (14 files OK, 2026-09-23).
- `grep -r secrets.` shows only `CARGO_REGISTRY_TOKEN`, `NPM_TOKEN`,
  optional `PYPI_API_TOKEN`; no secret values in repo (`secrets.yml` gate).
- `cargo package --list` green for all 18 crates (binding PR gate).
  `cargo publish --dry-run` is informational pre-first-release: it resolves
  internal deps against the crates.io index, so dependents fail until the
  first ordered publish lands (`continue-on-error: true` documented in workflow).
- `algo-redact` full `cargo publish --dry-run` (with verify) PASSED.
- Assumptions: crates.io token path is primary; trusted publishing for
  crates.io not assumed `[VERIFY-OPEN]` (re-check crates.io docs at first release);
  `pypa/gh-action-pypi-publish` OIDC fallback behavior re-verified at release.
