import {
  DiscordIcon,
  GithubIcon,
  Linkedin01Icon,
  NewTwitterIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { Link, NavLink } from "react-router-dom";
import { REPO_URL } from "../lib/site-url";

const PRODUCT_LINKS = [
  { to: "/features", label: "Features" },
  { to: "/how", label: "How it works" },
  { to: "/pricing", label: "Pricing" },
  { to: "/roadmap", label: "Roadmap" },
];

const RESOURCE_LINKS = [
  { to: "/docs", label: "Documentation" },
  { to: "/docs/install", label: "Install guide" },
  { to: "/docs/cli", label: "CLI reference" },
  { to: "/faq", label: "FAQ" },
  { to: "/privacy", label: "Privacy" },
];

function FooterLinkGroup({
  label,
  links,
}: {
  label: string;
  links: readonly { to: string; label: string }[];
}): JSX.Element {
  return (
    <div className="footer-impact-col">
      <p className="footer-impact-label">{label}</p>
      <nav className="footer-impact-links" aria-label={`Footer — ${label}`}>
        {links.map((l) => (
          <NavLink
            key={l.to}
            to={l.to}
            className={({ isActive }: { isActive: boolean }) =>
              isActive ? "is-active" : undefined
            }
          >
            {l.label}
          </NavLink>
        ))}
      </nav>
    </div>
  );
}

const SOCIALS = [
  { key: "github", label: "GitHub", href: REPO_URL, icon: GithubIcon },
  { key: "x", label: "X (Twitter)", href: "https://x.com", icon: NewTwitterIcon },
  {
    key: "linkedin",
    label: "LinkedIn",
    href: "https://linkedin.com",
    icon: Linkedin01Icon,
  },
  {
    key: "discord",
    label: "Discord",
    href: "https://discord.com",
    icon: DiscordIcon,
  },
];

export function Footer(): JSX.Element {
  return (
    <footer className="site-footer site-footer--impact">
      <div className="wrap footer-impact-inner">
        {/* Middle columns: brand + link groups */}
        <div className="footer-impact-mid">
          <div className="footer-impact-col footer-impact-brand">
            <Link
              to="/"
              className="footer-impact-brandmark"
              aria-label="Algorithco Guard — home"
            >
              <img
                src="/logo.svg"
                alt=""
                width={30}
                height={30}
                aria-hidden
                decoding="async"
              />
              <span>
                Algorithco <strong>Guard</strong>
              </span>
            </Link>
            <p className="footer-impact-desc">
              Local-first guardrails for AI coding agents. Your code stays on
              your machine.
            </p>
            <nav className="footer-impact-social" aria-label="Social">
              {SOCIALS.map((s) => (
                <a
                  key={s.key}
                  className={`footer-impact-social-link is-${s.key}`}
                  href={s.href}
                  target="_blank"
                  rel="noreferrer"
                  aria-label={s.label}
                  title={s.label}
                >
                  <HugeiconsIcon icon={s.icon} size={18} strokeWidth={1.8} />
                </a>
              ))}
            </nav>
          </div>
          <FooterLinkGroup label="Product" links={PRODUCT_LINKS} />
          <FooterLinkGroup label="Resources" links={RESOURCE_LINKS} />
        </div>

        {/* Bottom bar */}
        <div className="footer-impact-bottom">
          <span>© 2026 Algorithco</span>
          <Link to="/privacy">Privacy</Link>
        </div>
      </div>
    </footer>
  );
}
