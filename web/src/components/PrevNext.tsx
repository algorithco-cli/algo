import { Link } from "react-router-dom";
import { routeMeta } from "../lib/routes";

export function PrevNext({
  prev,
  next,
}: { prev?: string; next?: string }): JSX.Element | null {
  if (!prev && !next) return null;
  const prevMeta = prev ? routeMeta(prev) : undefined;
  const nextMeta = next ? routeMeta(next) : undefined;
  return (
    <nav className="prevnext" aria-label="Docs pagination">
      {prev && prevMeta ? (
        <Link to={prev} className="prevnext-prev">
          ← {prevMeta.title.split("—")[0].split("|")[0].trim()}
        </Link>
      ) : (
        <span />
      )}
      {next && nextMeta ? (
        <Link to={next} className="prevnext-next">
          {nextMeta.title.split("—")[0].split("|")[0].trim()} →
        </Link>
      ) : (
        <span />
      )}
    </nav>
  );
}
