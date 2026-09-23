import { Outlet } from "react-router-dom";
import { Breadcrumbs } from "./Breadcrumbs";
import { DocsSidebar } from "./DocsSidebar";

export function DocsLayout({
  trail,
}: { trail?: { label: string; to?: string }[] }): JSX.Element {
  return (
    <div className="wrap">
      <div className="docs-layout">
        <aside className="docs-side" aria-label="Docs navigation">
          <DocsSidebar />
        </aside>
        <div className="docs-content">
          {trail ? <Breadcrumbs trail={trail} /> : null}
          <Outlet />
        </div>
      </div>
    </div>
  );
}
