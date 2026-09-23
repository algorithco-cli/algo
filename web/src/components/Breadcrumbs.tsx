import { Fragment } from "react";
import { Link } from "react-router-dom";

export function Breadcrumbs({
  trail,
}: { trail: { label: string; to?: string }[] }): JSX.Element {
  return (
    <nav className="breadcrumbs" aria-label="Breadcrumb">
      {trail.map((t, i) => (
        <Fragment key={t.label}>
          {i > 0 ? <span aria-hidden> / </span> : null}
          {t.to && i < trail.length - 1 ? (
            <Link to={t.to}>{t.label}</Link>
          ) : (
            <span aria-current="page">{t.label}</span>
          )}
        </Fragment>
      ))}
    </nav>
  );
}
