import { readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { OG_IMAGE, SITE_URL } from "./site";

const here = dirname(fileURLToPath(import.meta.url));
const webRoot = join(here, "..", "..");

function read(p: string): string {
  return readFileSync(join(webRoot, p), "utf8");
}

function allSrcFiles(dir: string, out: string[] = []): string[] {
  for (const e of readdirSync(dir)) {
    const full = join(dir, e);
    if (statSync(full).isDirectory()) {
      if (e === "node_modules" || e === "dist") continue;
      allSrcFiles(full, out);
    } else if (/\.(ts|tsx|css)$/.test(e)) {
      out.push(full);
    }
  }
  return out;
}

describe("site canonical URL", () => {
  it("SITE_URL is local dev canonical, OG_IMAGE absolute", () => {
    expect(SITE_URL).toBe("http://127.0.0.1:3007");
    expect(OG_IMAGE).toBe("http://127.0.0.1:3007/og-image.png");
  });

  it("index.html og/twitter use absolute canonical, no placeholder domain", () => {
    const html = read("index.html");
    expect(html).toContain("http://127.0.0.1:3007/og-image.png");
    expect(html).toContain('property="og:url"');
    expect(html).not.toContain("algorithco.guard");
  });

  it("sitemap uses real endpoints (no fragments)", () => {
    const xml = read("public/sitemap.xml");
    for (const p of [
      "<loc>http://127.0.0.1:3007/</loc>",
      "<loc>http://127.0.0.1:3007/features</loc>",
      "<loc>http://127.0.0.1:3007/how</loc>",
      "<loc>http://127.0.0.1:3007/pricing</loc>",
      "<loc>http://127.0.0.1:3007/docs</loc>",
      "<loc>http://127.0.0.1:3007/docs/install</loc>",
      "<loc>http://127.0.0.1:3007/docs/cli</loc>",
      "<loc>http://127.0.0.1:3007/privacy</loc>",
      "<loc>http://127.0.0.1:3007/faq</loc>",
    ]) {
      expect(xml).toContain(p);
    }
    expect(xml).not.toMatch(/<loc>[^<]*#/);
    expect(xml).not.toContain("algorithco.guard");
  });

  it("route manifest has SEO for every endpoint", () => {
    const routesSrc = read("src/lib/routes.ts");
    for (const p of [
      "/features",
      "/how",
      "/pricing",
      "/docs/install",
      "/docs/cli",
      "/privacy",
      "/faq",
    ]) {
      expect(routesSrc).toContain(`path: "${p}"`);
    }
    expect(routesSrc).toContain('path: "/"');
  });

  it("robots.txt sitemap is absolute", () => {
    const txt = read("public/robots.txt");
    expect(txt).toContain("Sitemap: http://127.0.0.1:3007/sitemap.xml");
  });
});

describe("token hex hygiene (Variant 1)", () => {
  it("no hard-coded hex outside Tokens.css", () => {
    const files = allSrcFiles(join(webRoot, "src"));
    const hex = /#[0-9a-fA-F]{6}\b/;
    const offenders: string[] = [];
    // Allowed exceptions: component-local visual effects that are not part of Variant 1 palette
    const allow = new Set([
      "src/components/BranchedMenu.css",
      "src/components/ElectricBorder.tsx",
      "src/components/Grainient.tsx",
      "src/components/FolderFloat.css",
      "src/components/FolderFloat.tsx",
      "src/components/PlatformIcons.tsx", // third-party brand marks (Google "G" must keep its official colors)
      "src/components/PixelGuardDog.tsx", // pixel-sprite palette (fixed artwork, not the Variant 1 theme)
      "src/pages/Home.tsx",
      "src/pages/Pricing.tsx",
    ]);
    for (const f of files) {
      const norm = f.replace(/\\/g, "/");
      if (norm.endsWith("components/Tokens.css")) continue;
      if (norm.endsWith(".test.ts")) continue;
      const rel = norm.split("web/")[1] ?? norm;
      if (allow.has(rel)) continue;
      const src = readFileSync(f, "utf8");
      // Allow href="#..." anchors (not colors) — strip them before check.
      const stripped = src
        .replace(/href="#[^"]*"/g, 'href=""')
        .replace(/href='#[^']*'/g, "href=''");
      if (hex.test(stripped)) offenders.push(f);
    }
    expect(offenders).toEqual([]);
  });

  it("Tokens.css core values match canonical design-tokens.css", () => {
    // Dark-only palette (light theme deleted): dark values must exist in both.
    const tokens = read("src/components/Tokens.css").toLowerCase();
    const canonical = read("design-tokens.css").toLowerCase();
    for (const v of ["#8e77ff", "#ff6b6f", "#9c9ab0"]) {
      expect(tokens).toContain(v);
      expect(canonical).toContain(v);
    }
    // No light-theme values may remain in either file.
    for (const v of ["#6d4aff", "#fafafb", "#e5484d", "#d99a00", "#6b6a7b"]) {
      expect(tokens).not.toContain(v);
      expect(canonical).not.toContain(v);
    }
  });
});

describe("privacy: no auto-fetch on load", () => {
  it("stars load only on explicit opt-in (no fetch in App shell)", () => {
    const app = read("src/App.tsx");
    const stars = read("src/components/GithubStars.tsx");
    expect(app).not.toMatch(/api\.github\.com/);
    expect(stars).toContain("useGithubStarsOptIn");
    expect(stars).toContain("api.github.com");
  });
});
