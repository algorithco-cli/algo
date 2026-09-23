import ArrowRight from "lucide-react/icons/arrow-right.mjs";
import { Link } from "react-router-dom";

export function Cta(): JSX.Element {
  return (
    <div className="cta reveal">
      <div className="cta-glow" aria-hidden />
      <div className="cta-icon" aria-hidden>
        <img
          src="/logo.svg"
          alt=""
          width={40}
          height={40}
          loading="lazy"
          decoding="async"
        />
      </div>
      <div className="cta-copy">
        <p className="kicker">Get started</p>
        <h2>Ship agents you don&apos;t have to babysit.</h2>
        <p className="muted">
          Free local MVP. ~30s install. Shadow-first, reversible, explained.
        </p>
      </div>
      <div className="cta-actions">
        <Link to="/docs/install" className="btn btn-primary btn-lg">
          Install free <ArrowRight size={16} aria-hidden />
        </Link>
        <Link to="/login" className="btn btn-ghost btn-lg">
          Log in
        </Link>
      </div>
    </div>
  );
}
