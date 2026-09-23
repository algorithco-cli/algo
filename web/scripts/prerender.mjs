// web/scripts/prerender.mjs — static prerender for real endpoints.
// After `vite build`: copies dist/index.html → dist/<route>/index.html with
// per-page title/description/canonical/og:url replaced, writes dist/404.html
// + dist/sitemap.xml from src/lib/routes.ts (single manifest).
// No SSR renderToString in MVP (CSR shell + correct head = indexable titles,
// unfurlable cards). Full string render is phase 2.
import {
  cpSync,
  existsSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const webRoot = join(here, "..");
const dist = join(webRoot, "dist");

const SITE_URL = "http://127.0.0.1:3007";

const PAGES = [
  {
    path: "/",
    title: "Algorithco Guard — safer AI coding agents",
    desc: "Control layer for AI coding agents. Hard-deny destructive commands, fail-safe ask, redact-first privacy. Free local MVP, ~30s install.",
  },
  {
    path: "/features",
    title: "Features — safer, quieter, auditable guardrails | Algorithco Guard",
    desc: "13 hard-deny rules, fail-safe ask on every error, shadow-first rollout, algo why audit. Under 3ms local decisions.",
  },
  {
    path: "/how",
    title: "How it works — hook to audit in milliseconds | Algorithco Guard",
    desc: "Hook client, on-machine parse+redact, L0/L1/L3 decision, SQLite audit. Fail-safe ask, redacted egress inspector.",
  },
  {
    path: "/pricing",
    title: "Pricing — free local, team + enterprise planned | Algorithco Guard",
    desc: "Local MVP free forever. Team $19/seat/mo and Enterprise custom planned for Phase 3 cloud with dashboard and SSO.",
  },
  {
    path: "/roadmap",
    title: "Roadmap — local MVP to cloud + expansion | Algorithco Guard",
    desc: "P1 local done, P2 enforcement next, P3 cloud+teams, P4 Codex/OpenCode, scanner, TUI, Windows. Gates per phase.",
  },
  {
    path: "/login",
    title: "Log in — link terminal via device code | Algorithco Guard",
    desc: "algo login device flow demo. Team cloud with org policy and dashboard is Phase 3 planned; local MVP runs today.",
    noindex: true,
  },
  {
    path: "/faq",
    title: "FAQ — blocking, agents, privacy, speed, cost | Algorithco Guard",
    desc: "Does Guard block commands? Which agents? Does code leave? How fast? Cost? Open source? Shadow-first answers.",
  },
  {
    path: "/privacy",
    title:
      "Privacy — local-only by default, BYOK redacted opt-in | Algorithco Guard",
    desc: "local-only sends nothing. BYOK redacted/full sends to TypeSafe US infra, no training on Input, as-long-as-necessary retention. Inspect with algo log.",
  },
  {
    path: "/docs",
    title: "Docs — install once, know every command | Algorithco Guard",
    desc: "One binary, nine commands. Additive install, byte-identical uninstall. Start with install, then CLI reference.",
  },
  {
    path: "/docs/install",
    title: "Install — algo init in ~30s (Claude Code now) | Algorithco Guard",
    desc: "curl install.sh, algo init diff+consent, doctor verify. Codex/OpenCode adapters Phase 4. Verify-before-run included.",
  },
  {
    path: "/docs/cli",
    title:
      "CLI reference — algo init/doctor/status/why/log/enforce | Algorithco Guard",
    desc: "Nine commands: init, doctor, status, why (action+reason+confidence+source+latency), log --show-egress, enforce, pause, login, policy.",
  },
];

function canonical(path) {
  return path === "/" ? `${SITE_URL}/` : `${SITE_URL}${path}`;
}

function esc(s) {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/"/g, "&quot;");
}

const shell = readFileSync(join(dist, "index.html"), "utf8");
const written = [];

for (const p of PAGES) {
  let html = shell;
  html = html.replace(/<title>.*?<\/title>/, `<title>${esc(p.title)}</title>`);
  if (/<meta name="description"/.test(html)) {
    html = html.replace(
      /<meta name="description" content="[^"]*"/,
      `<meta name="description" content="${esc(p.desc)}"`,
    );
  }
  html = html.replace(
    /<meta property="og:url" content="[^"]*"/,
    `<meta property="og:url" content="${canonical(p.path)}"`,
  );
  html = html.replace(
    /<meta property="og:title" content="[^"]*"/,
    `<meta property="og:title" content="${esc(p.title)}"`,
  );
  html = html.replace(
    /<meta property="og:description" content="[^"]*"/,
    `<meta property="og:description" content="${esc(p.desc)}"`,
  );
  if (/<link rel="canonical"/.test(html)) {
    html = html.replace(
      /<link rel="canonical" href="[^"]*"/,
      `<link rel="canonical" href="${canonical(p.path)}"`,
    );
  } else {
    html = html.replace(
      "</head>",
      `  <link rel="canonical" href="${canonical(p.path)}" />\n  </head>`,
    );
  }
  if (p.noindex && !/<meta name="robots"/.test(html)) {
    html = html.replace(
      "</head>",
      `  <meta name="robots" content="noindex,follow" />\n  </head>`,
    );
  }
  if (p.path === "/") {
    writeFileSync(join(dist, "index.html"), html);
    written.push("/");
    continue;
  }
  const dir = join(dist, p.path.slice(1));
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, "index.html"), html);
  written.push(p.path);
}

// 404 fallback (hosts + vite preview).
cpSync(join(dist, "index.html"), join(dist, "404.html"));

// sitemap.xml from same manifest (absolute URLs, no fragments).
const sitemap = `<?xml version="1.0" encoding="UTF-8"?>\n<!-- Generated by scripts/prerender.mjs from src/lib/routes.ts. Canonical: ${SITE_URL}. Production domain TBD. -->\n<urlset xmlns="http://www.sitemap.org/schemas/sitemap/0.9">\n${PAGES.filter(
  (p) => !p.noindex,
)
  .map((p) => `  <url><loc>${canonical(p.path)}</loc></url>`)
  .join("\n")}\n</urlset>\n`;
writeFileSync(join(dist, "sitemap.xml"), sitemap);
if (existsSync(join(webRoot, "public", "sitemap.xml"))) {
  writeFileSync(join(webRoot, "public", "sitemap.xml"), sitemap);
}

console.log(
  `prerender: ${written.length} routes → dist/*/index.html + 404.html + sitemap.xml`,
);
