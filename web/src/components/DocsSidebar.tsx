import { NavLink } from "react-router-dom";
import { DOCS_SIDEBAR } from "../lib/routes";

export function DocsSidebar(): JSX.Element {
  return (
    <nav className="docs-nav" aria-label="Documentation">
      {DOCS_SIDEBAR.map((group) => (
        <div className="docs-nav-group" key={group.label}>
          <p className="docs-nav-label">{group.label}</p>
          <div className="docs-nav-links">
            {group.children.map((item) => (
              <NavLink
                key={item.value}
                to={item.to}
                end={item.to === "/docs"}
                className={({ isActive }) =>
                  `docs-nav-link${isActive ? " is-active" : ""}`
                }
              >
                <span>{item.label}</span>
                <span className="docs-nav-active-mark" aria-hidden="true" />
              </NavLink>
            ))}
          </div>
        </div>
      ))}
    </nav>
  );
}
