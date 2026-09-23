import { Link } from "react-router-dom";
import { LoginMock } from "../components/LoginMock";
import { Section } from "../components/Section";
import { Seo } from "../components/Seo";
import { useRevealOnMount } from "../hooks/useRevealOnMount";

export default function Login(): JSX.Element {
  useRevealOnMount();
  return (
    <>
      <Seo path="/login" />
      <Section
        id="login"
        kicker="Log in"
        title="Link your terminal"
        accent="in seconds"
        lede="OAuth device flow — no passwords in the terminal. Demo, stays on this page."
      >
        <LoginMock />
        <p className="muted small">
          Team cloud is Phase 3 (planned); today everything runs local-first.
          Get started local <Link to="/docs/install">here</Link> · team pricing{" "}
          <Link to="/pricing">here</Link>.
        </p>
      </Section>
    </>
  );
}
