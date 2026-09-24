# Release — set secrets once → tag → auto-publish

> Status: automation ready, publishing **blocked** until legal/license (ADR-0001),
> public repo, and first-release checklist below. Dry-runs run on every PR with
> no secrets. Real publish needs the `release` environment + tag.

## 1. One-time setup (owner, ~15 min)

### GitHub Environment `release`

1. Repo → Settings → Environments → New environment → name `release`.
2. Required reviewers: add CODEOWNERS humans (license/deny/policy/sig-verify owners).
3. (Recommended) Deployment branches: only `release/*` tags.

### Secrets (repo → Settings → Secrets and variables → Actions)

Scope secrets to environment `release` (not repo-wide):

| Secret | Where to get it | Notes |
|---|---|---|
| `CARGO_REGISTRY_TOKEN` | crates.io → Account Settings → API Tokens → New Token, scope `publish` | Used by `publish-crates.yml`. |
| `NPM_TOKEN` | npmjs.com → Access Tokens → Generate New Token → **Automation**; grant push on `@algorithco` scope | Used by `publish-npm.yml` with `--provenance`. |
| `PYPI_API_TOKEN` | **Fallback only.** Prefer Trusted Publisher below. PyPI → Account → API tokens, scope `algorithco-guard-eval` | Used only if OIDC unavailable. |

### PyPI Trusted Publisher (preferred, no secret)

1. Create PyPI project `algorithco-guard-eval` (or reserve the name).
2. Project → Settings → Trusted Publisher → Add GitHub publisher:
   owner `algorithcoguard`, repo `algorithco-guard`,
   workflow `publish-pypi.yml`, environment `release`.
3. Workflow uses `pypa/gh-action-pypi-publish` with `id-token: write`; when
   `PYPI_API_TOKEN` is absent it publishes via OIDC.

### npm scope

Reserve `@algorithco` org on npmjs.com and grant the token push access,
or change `packages/contracts/package.json` name before first publish.

## 2. Pre-release checklist (must all pass)

- [ ] ADR-0001 license signed → replace `LicenseRef-TBD-legal-signoff` + add `LICENSE` files.
- [ ] Repo public (or registries accept private-repo links) — links currently 404.
- [ ] `algo-types` proto-vendoring fixed (`build.rs` reads `../../../proto`,
      outside the crate tarball — `cargo publish --verify` fails; vendor protos
      or generated code before first crates release).
- [ ] Versions bumped consistently (`0.1.0` → real first version across manifests).
- [ ] Dry-run jobs green on the release PR.

## 3. Publish

```powershell
git tag release/v0.1.0
git push origin release/v0.1.0
# → Actions: Publish crates / Publish PyPI / Publish npm (approve `release` env)
```

Manual re-run: Actions → workflow → Run workflow → `confirm=publish`
(dry-run is the default; real publish requires the tag or explicit confirm).

Order is topological for crates (workflow handles it):
types → redact/shell-analysis/hook-client/backend → fingerprint/policy/audit →
provider/adapters → daemon/cli/verifier/loop/scanner/tui.
`algo-policy-spike` + `fuzz` never publish (`publish=false`).

Rollback: registries are immutable — yank (`cargo yank`), delete nothing.
Fix forward with a patch version tag.

## 4. What runs where

| Workflow | Dry-run (PR/push main, no secrets) | Real (tag/dispatch, `release` env) |
|---|---|---|
| `publish-crates.yml` | `cargo package --list` + `cargo publish --dry-run` in order | `cargo publish` with `CARGO_REGISTRY_TOKEN` |
| `publish-pypi.yml` | `python -m build` + `twine check` | OIDC (or token) upload + attest |
| `publish-npm.yml` | asserts dashboard/web `private:true`, `npm pack --dry-run` contracts | `npm publish --provenance --access public` contracts only |
