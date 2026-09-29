// ─────────────────────────────────────────────────────────────
// EDIT ME — the only things you should need to change.
// ─────────────────────────────────────────────────────────────
export const CONFIG = {
  // The repo's install page currently uses the local dev URL below
  // (the production domain is still "TBD" in web/src/lib/site.ts).
  // Put your real domain here once it exists, e.g. "https://algorithco.dev"
  siteUrl: 'http://127.0.0.1:3007',

  // Show the site URL under the final call-to-action button?
  showUrlInCta: false,
  ctaLabel: 'Install free',
} as const;

export const INSTALL_CMD = `curl -fsSL ${CONFIG.siteUrl}/install.sh | sh`;
