import { Link } from "react-router-dom";
import { Seo } from "../components/Seo";

export default function NotFound(): JSX.Element {
  return (
    <>
      <Seo path="/404" />
      <section className="section reveal visible">
        <div className="wrap">
          <p className="kicker">404</p>
          <h2 className="section-title">
            Lost in the <span className="accent">pipeline?</span>
          </h2>
          <p className="muted section-lede">
            That path returned ask — here&apos;s where to go.
          </p>
          <div className="section-body">
            <p>
              <Link to="/" className="btn btn-primary">
                Go home
              </Link>{" "}
              <Link to="/docs" className="btn btn-ghost">
                Docs
              </Link>{" "}
              <Link to="/privacy" className="btn btn-ghost">
                Privacy
              </Link>{" "}
              <Link to="/faq" className="btn btn-ghost">
                FAQ
              </Link>
            </p>
            <p className="muted small">
              Still stuck in the terminal? Try <code>algo doctor</code>.
            </p>
          </div>
        </div>
      </section>
    </>
  );
}
