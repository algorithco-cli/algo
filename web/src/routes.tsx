import * as React from "react";
import { Route, Routes } from "react-router-dom";
import { DashboardLayout } from "./components/DashboardLayout";
import { DocsLayout } from "./components/DocsLayout";
import { MarketingLayout } from "./components/MarketingLayout";
import { RootLayout } from "./components/RootLayout";

const Home = React.lazy(() => import("./pages/Home"));
const Features = React.lazy(() => import("./pages/Features"));
const How = React.lazy(() => import("./pages/How"));
const Pricing = React.lazy(() => import("./pages/Pricing"));
const Billing = React.lazy(() => import("./pages/Billing"));
const Roadmap = React.lazy(() => import("./pages/Roadmap"));
const Login = React.lazy(() => import("./pages/Login"));
const Faq = React.lazy(() => import("./pages/Faq"));
const Privacy = React.lazy(() => import("./pages/Privacy"));
const Terms = React.lazy(() => import("./pages/Terms"));
const Security = React.lazy(() => import("./pages/Security"));
const DocsIndex = React.lazy(() => import("./pages/docs/DocsIndex"));
const Install = React.lazy(() => import("./pages/docs/Install"));
const Cli = React.lazy(() => import("./pages/docs/Cli"));
const Configuration = React.lazy(() => import("./pages/docs/Configuration"));
const PrivacySecurity = React.lazy(
  () => import("./pages/docs/PrivacySecurity"),
);
const Troubleshooting = React.lazy(
  () => import("./pages/docs/Troubleshooting"),
);
const Architecture = React.lazy(() => import("./pages/docs/Architecture"));
const DashboardOverview = React.lazy(
  () => import("./pages/dashboard/Overview"),
);
const DashboardHistory = React.lazy(() => import("./pages/dashboard/History"));
const DashboardAudit = React.lazy(() => import("./pages/dashboard/Audit"));
const DashboardPolicy = React.lazy(() => import("./pages/dashboard/Policy"));
const DashboardStats = React.lazy(() => import("./pages/dashboard/Stats"));
const NotFound = React.lazy(() => import("./pages/NotFound"));

export function AppRoutes(): JSX.Element {
  return (
    <React.Suspense
      fallback={
        <output className="route-loader" aria-live="polite">
          Loading page…
        </output>
      }
    >
      <Routes>
        <Route element={<RootLayout />}>
          <Route element={<MarketingLayout />}>
            <Route index element={<Home />} />
            <Route path="features" element={<Features />} />
            <Route path="how" element={<How />} />
            <Route path="pricing" element={<Pricing />} />
            <Route path="billing" element={<Billing />} />
            <Route path="roadmap" element={<Roadmap />} />
            <Route path="login" element={<Login />} />
            <Route path="faq" element={<Faq />} />
            <Route path="privacy" element={<Privacy />} />
            <Route path="terms" element={<Terms />} />
            <Route path="security" element={<Security />} />
          </Route>
          <Route path="docs" element={<DocsLayout />}>
            <Route index element={<DocsIndex />} />
            <Route path="install" element={<Install />} />
            <Route path="cli" element={<Cli />} />
            <Route path="configuration" element={<Configuration />} />
            <Route path="privacy-security" element={<PrivacySecurity />} />
            <Route path="troubleshooting" element={<Troubleshooting />} />
            <Route path="architecture" element={<Architecture />} />
          </Route>
          {/* Team dashboard (dashboard plan). Client-only: excluded from prerender
              + sitemap. */}
          <Route path="dashboard" element={<DashboardLayout />}>
            <Route index element={<DashboardOverview />} />
            <Route path="history" element={<DashboardHistory />} />
            <Route path="audit" element={<DashboardAudit />} />
            <Route path="policy" element={<DashboardPolicy />} />
            <Route path="stats" element={<DashboardStats />} />
          </Route>
          <Route path="*" element={<NotFound />} />
        </Route>
      </Routes>
    </React.Suspense>
  );
}
