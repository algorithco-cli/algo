import { Outlet, useLocation } from "react-router-dom";
import { useRevealOnMount } from "../hooks/useRevealOnMount";
import { Footer } from "./Footer";
import { Header } from "./Header";
import { ScrollToTop } from "./ScrollToTop";

export function RootLayout(): JSX.Element {
  useRevealOnMount();
  // Login-only footer treatment: scope via data-page on the page wrapper
  // and a matching Footer variant — global footer styles stay untouched.
  const isLogin = useLocation().pathname === "/login";
  // .page and <main> are full-bleed (width:100% / max-width:none). Do NOT
  // wrap <Outlet> in .wrap here — per-section inner .wrap is the only
  // centered 1240px container. Wrapping the outlet globally reintroduces
  // the right-gap bug for every section including footer.
  return (
    <div className="page" data-page={isLogin ? "login" : "default"}>
      <a href="#main" className="skip-link">
        Skip to content
      </a>
      <ScrollToTop />
      <Header />
      <main id="main">
        <Outlet />
      </main>
      <Footer variant={isLogin ? "transparent" : "default"} />
    </div>
  );
}
