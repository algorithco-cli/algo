import {
  DiscordIcon,
  GithubIcon,
  Linkedin01Icon,
  NewTwitterIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { Link } from "react-router-dom";
import { REPO_URL } from "../lib/site-url";

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
        {/* Middle columns */}
        <div className="footer-impact-mid">
          <div className="footer-impact-col">
            <p className="footer-impact-label">ALGORITHCO</p>
            <p className="footer-impact-desc">
              Local-first AI agent guardrails.
              <br />
              by Algorithco
              <br />
              <a href="/docs">Docs</a> · <a href="/privacy">Privacy</a>
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
          <div className="footer-impact-col">
            <p className="footer-impact-label">NAVIGATION</p>
            <nav className="footer-impact-links" aria-label="Footer navigation">
              <Link to="/">Home</Link>
              <Link to="/features">Features</Link>
              <Link to="/how">How it works</Link>
              <Link to="/docs">Docs</Link>
              <Link to="/pricing">Pricing</Link>
              <Link to="/faq">Contact</Link>
            </nav>
          </div>
        </div>

        {/* Bottom bar */}
        <div className="footer-impact-bottom">
          <span>© 2026 Algorithco</span>
          <span className="footer-impact-legal">
            <Link to="/privacy">Legal</Link> —{" "}
            <Link to="/privacy">Privacy</Link> —{" "}
            <Link to="/privacy">Cookies</Link>
          </span>
        </div>
      </div>
    </footer>
  );
}
