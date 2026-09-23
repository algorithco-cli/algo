import * as React from "react";
import { Route, Routes } from "react-router-dom";
import { DocsLayout } from "./components/DocsLayout";
import { MarketingLayout } from "./components/MarketingLayout";
import { RootLayout } from "./components/RootLayout";

const Home = React.lazy(() => import("./pages/Home"));
const Features = React.lazy(() => import("./pages/Features"));
const How = React.lazy(() => import("./pages/How"));
const Pricing = React.lazy(() => import("./pages/Pricing"));
const Roadmap = React.lazy(() => import("./pages/Roadmap"));
const Login = React.lazy(() => import("./pages/Login"));
const Faq = React.lazy(() => import("./pages/Faq"));
const Privacy = React.lazy(() => import("./pages/Privacy"));
const DocsIndex = React.lazy(() => import("./pages/docs/DocsIndex"));
const Install = React.lazy(() => import("./pages/docs/Install"));
const Cli = React.lazy(() => import("./pages/docs/Cli"));
const NotFound = React.lazy(() => import("./pages/NotFound"));

export function AppRoutes(): JSX.Element {
  return (
    <React.Suspense fallback={null}>
      <Routes>
        <Route element={<RootLayout />}>
          <Route element={<MarketingLayout />}>
            <Route index element={<Home />} />
            <Route path="features" element={<Features />} />
            <Route path="how" element={<How />} />
            <Route path="pricing" element={<Pricing />} />
            <Route path="roadmap" element={<Roadmap />} />
            <Route path="login" element={<Login />} />
            <Route path="faq" element={<Faq />} />
            <Route path="privacy" element={<Privacy />} />
          </Route>
          <Route path="docs" element={<DocsLayout />}>
            <Route index element={<DocsIndex />} />
            <Route path="install" element={<Install />} />
            <Route path="cli" element={<Cli />} />
          </Route>
          <Route path="*" element={<NotFound />} />
        </Route>
      </Routes>
    </React.Suspense>
  );
}
