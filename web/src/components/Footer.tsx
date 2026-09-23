import ArrowUp from "lucide-react/icons/arrow-up.mjs";
import { Link } from "react-router-dom";
import { REPO_URL } from "../lib/site-url";

export function Footer(): JSX.Element {
  const scrollTop = (): void => {
    const reduce = window.matchMedia?.(
      "(prefers-reduced-motion: reduce)",
    ).matches;
    window.scrollTo({ top: 0, behavior: reduce ? "auto" : "smooth" });
  };

  return (
    <footer className="site-footer site-footer--impact">
      <div className="wrap footer-impact-inner">
        {/* Top row */}
        <div className="footer-impact-top">
          <h2 className="footer-impact-tagline">
            AGENTS YOU TRUST.
            <br />
            GUARDED BY DESIGN.
          </h2>
          <button
            type="button"
            className="footer-impact-backtop"
            onClick={scrollTop}
            aria-label="Back to top"
          >
            <ArrowUp size={18} aria-hidden />
          </button>
        </div>

        {/* Middle 3 columns */}
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
          <div className="footer-impact-col">
            <p className="footer-impact-label">FOLLOW</p>
            <nav className="footer-impact-links" aria-label="Social">
              <a href={REPO_URL} target="_blank" rel="noreferrer">
                GitHub
              </a>
              <a href="https://x.com" target="_blank" rel="noreferrer">
                X (Twitter)
              </a>
              <a href="https://linkedin.com" target="_blank" rel="noreferrer">
                LinkedIn
              </a>
              <a href="https://discord.com" target="_blank" rel="noreferrer">
                Discord
              </a>
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
