// web/scripts/sync-deny-list.mjs — parity check: core deny_list.rs vs src/lib/verdict.ts
// Usage: `npm run gen:deny` — exits non-zero on ID drift. No network, no writes.
// Canonical: core/crates/policy/src/deny_list.rs HARD_DENY_RULES
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const webRoot = join(here, "..");
const coreDeny = join(
  webRoot,
  "..",
  "core",
  "crates",
  "policy",
  "src",
  "deny_list.rs",
);
const verdictTs = join(webRoot, "src", "lib", "verdict.ts");

function extractRustIds(src) {
  const m = src.match(/HARD_DENY_RULES[^=]*=\s*&\[([\s\S]*?)\];/);
  if (!m) throw new Error("HARD_DENY_RULES block not found in deny_list.rs");
  return [...m[1].matchAll(/"([A-Z0-9_]+)"/g)].map((x) => x[1]);
}

function extractTsIds(src) {
  const m = src.match(/HARD_DENY_RULE_IDS[^=]*=\s*\[([\s\S]*?)\]\s*as const/);
  if (!m) throw new Error("HARD_DENY_RULE_IDS block not found in verdict.ts");
  return [...m[1].matchAll(/"([A-Z0-9_]+)"/g)].map((x) => x[1]);
}

const rustSrc = readFileSync(coreDeny, "utf8");
const tsSrc = readFileSync(verdictTs, "utf8");
const rustIds = extractRustIds(rustSrc);
const tsIds = extractTsIds(tsSrc);

const missing = rustIds.filter((id) => !tsIds.includes(id));
const extra = tsIds.filter((id) => !rustIds.includes(id));

console.log(`core (${coreDeny}): ${rustIds.length} rules`);
console.log(`web  (${verdictTs}): ${tsIds.length} rules`);
if (missing.length || extra.length) {
  if (missing.length) console.error(`MISSING in web: ${missing.join(", ")}`);
  if (extra.length)
    console.error(`EXTRA in web (not in core): ${extra.join(", ")}`);
  console.error(
    "DRIFT: update web/src/lib/verdict.ts to match core, then re-run vitest.",
  );
  process.exit(1);
}
console.log(
  "OK: rule IDs in parity (13). Patterns are JS ports — review regex diffs manually.",
);
