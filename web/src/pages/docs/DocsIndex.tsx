import ArrowRight from "lucide-react/icons/arrow-right.mjs";
import Download from "lucide-react/icons/download.mjs";
import Shield from "lucide-react/icons/shield.mjs";
import Terminal from "lucide-react/icons/terminal.mjs";
import Zap from "lucide-react/icons/zap.mjs";
import { Link } from "react-router-dom";
import { Breadcrumbs } from "../../components/Breadcrumbs";
import { CopyButton } from "../../components/CopyButton";
import { Seo } from "../../components/Seo";
import { useRevealOnMount } from "../../hooks/useRevealOnMount";

export default function DocsIndex(): JSX.Element {
  useRevealOnMount();
  return (
    <>
      <Seo path="/docs" />
      <Breadcrumbs trail={[{ label: "Home", to: "/" }, { label: "Docs" }]} />

      {/* Hero */}
      <div className="docs-hero reveal">
        <p className="kicker">Docs</p>
        <h1 className="docs-hero-title">
          Install once, <span className="accent">know every command</span>
        </h1>
        <p className="muted docs-hero-lede">
          One binary, nine commands. Additive install, byte-identical uninstall.
          Start in ~30s, verify before you trust.
        </p>
      </div>

      {/* Quick start */}
      <div className="card docs-quickstart reveal">
        <div className="docs-quickstart-head">
          <span className="docs-qs-icon" aria-hidden>
            <Zap size={16} />
          </span>
          <strong>Quick start</strong>
          <span className="muted small">
            ~30s · Claude Code · local-only by default
          </span>
        </div>
        <pre className="docs-qs-code">
          <code>{`curl -fsSL http://127.0.0.1:3007/install.sh | sh
algo init   # detect agents → diff → consent → hooks`}</code>
        </pre>
        <div className="docs-qs-actions">
          <CopyButton text="curl -fsSL http://127.0.0.1:3007/install.sh | sh" />
          <Link to="/docs/install" className="btn btn-primary">
            Full install guide <ArrowRight size={14} aria-hidden />
          </Link>
        </div>
      </div>

      {/* Primary navigation */}
      <div className="docs-hub-grid reveal">
        <Link to="/docs/install" className="card docs-hub-card">
          <span className="docs-hub-icon" aria-hidden>
            <Download size={18} />
          </span>
          <h3>Install</h3>
          <p className="muted">
            algo init in ~30s. Claude Code now; Codex / OpenCode planned.
            Verify-before-run, per-agent consent, byte-identical backup.
          </p>
          <span className="docs-hub-link">
            Install <ArrowRight size={14} aria-hidden />
          </span>
        </Link>
        <Link to="/docs/cli" className="card docs-hub-card">
          <span className="docs-hub-icon" aria-hidden>
            <Terminal size={18} />
          </span>
          <h3>CLI reference</h3>
          <p className="muted">
            Nine commands: init, doctor, status, why, log, enforce, pause,
            login, policy. Every flag, exit code, and example.
          </p>
          <span className="docs-hub-link">
            CLI reference <ArrowRight size={14} aria-hidden />
          </span>
        </Link>
        <Link to="/privacy" className="card docs-hub-card">
          <span className="docs-hub-icon" aria-hidden>
            <Shield size={18} />
          </span>
          <h3>Privacy</h3>
          <p className="muted">
            Local-only default, redacted / full BYOK opt-in. Where data goes,
            how long, how to inspect with --show-egress.
          </p>
          <span className="docs-hub-link">
            Privacy <ArrowRight size={14} aria-hidden />
          </span>
        </Link>
      </div>

      {/* Secondary links */}
      <div className="docs-hub-meta reveal">
        <div className="card docs-meta-card">
          <h4>Next step</h4>
          <p className="muted small">
            New here? Start with the install guide, then open the CLI reference.
          </p>
          <Link to="/docs/install">Install →</Link>
        </div>
        <div className="card docs-meta-card">
          <h4>Trust</h4>
          <p className="muted small">
            Modes, subprocessors, retention, redact-before-network — all
            verbatim from the privacy doc.
          </p>
          <Link to="/how">How it works →</Link>
        </div>
        <div className="card docs-meta-card">
          <h4>Support</h4>
          <p className="muted small">
            Blocked today? Which agents? Does code leave? Answered plainly.
          </p>
          <Link to="/faq">FAQ →</Link>
        </div>
      </div>
    </>
  );
}
