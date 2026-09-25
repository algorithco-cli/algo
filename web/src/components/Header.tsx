import Menu from "lucide-react/icons/menu.mjs";
import X from "lucide-react/icons/x.mjs";
import * as React from "react";
import { Link, NavLink, useLocation } from "react-router-dom";
import { useScrolled } from "../hooks/useScrolled";
import { NAV_LINKS } from "../lib/routes";

export function Header(): JSX.Element {
  const scrolled = useScrolled();
  const [menuOpen, setMenuOpen] = React.useState(false);
  const pathname = useLocation().pathname;
  // biome-ignore lint/correctness/useExhaustiveDependencies: close mobile menu on route change — effect intentionally keyed on pathname
  React.useEffect(() => {
    setMenuOpen(false);
  }, [pathname]);
  const close = React.useCallback(() => setMenuOpen(false), []);
  const toggleMenu = React.useCallback(() => setMenuOpen((o) => !o), []);
  return (
    <header className={`site-header${scrolled ? " scrolled" : ""}`}>
      <div className="wrap header-inner">
        <Link to="/" className="brand" aria-label="Algorithco Guard — home">
          <img
            src="/logo.svg"
            alt=""
            width={38}
            height={38}
            className="brand-logo"
            aria-hidden
            fetchPriority="high"
            decoding="async"
          />
          <span className="brand-text">
            <span className="brand-name">
              Algorithco <span className="brand-accent">Guard</span>
            </span>
            <span className="brand-by">
              by <strong>Algorithco</strong>
            </span>
          </span>
        </Link>
        <nav className="nav-desktop" aria-label="Primary">
          {NAV_LINKS.map(({ label, to }) => (
            <NavLink
              key={to}
              to={to}
              className={({ isActive }) => (isActive ? "active" : undefined)}
            >
              {label}
            </NavLink>
          ))}
        </nav>
        <div className="header-cta">
          <Link to="/login" className="btn btn-ghost cta-login">
            Log in
          </Link>
          <Link to="/docs/install" className="btn btn-primary">
            Get started
          </Link>
          <button
            type="button"
            className="burger"
            onClick={toggleMenu}
            aria-expanded={menuOpen}
            aria-label={menuOpen ? "Close navigation" : "Open navigation"}
          >
            {menuOpen ? (
              <X size={22} aria-hidden />
            ) : (
              <Menu size={22} aria-hidden />
            )}
          </button>
        </div>
      </div>
      {menuOpen ? (
        <nav className="mobile-menu" aria-label="Mobile">
          {NAV_LINKS.map(({ label, to }) => (
            <NavLink key={to} to={to} onClick={close}>
              {label}
            </NavLink>
          ))}
          <div className="mobile-actions">
            <Link to="/login" className="btn btn-ghost" onClick={close}>
              Log in
            </Link>
            <Link
              to="/docs/install"
              className="btn btn-primary"
              onClick={close}
            >
              Get started
            </Link>
          </div>
        </nav>
      ) : null}
    </header>
  );
}
