/* web — canonical site URL (single source for sitemap/og/robots checks).
 * Private MVP: local dev canonical is http://127.0.0.1:3007.
 * Production domain TBD — update SITE_URL + index.html + public/sitemap.xml
 * + public/robots.txt together when the domain lands (see ADR-0011).
 */
export const SITE_URL = "http://127.0.0.1:3007";
export const OG_IMAGE = `${SITE_URL}/og-image.png`;

export function canonicalFor(path: string): string {
  return path === "/" ? `${SITE_URL}/` : `${SITE_URL}${path}`;
}
