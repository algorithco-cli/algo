import { Outlet } from "react-router-dom";

/* MarketingLayout must NOT wrap in .wrap — that was the single root cause
   of the recurring full-width bug (global .wrap around all sections left a
   right gap at every viewport). Each section/header/footer provides its OWN
   inner .wrap for centered 1240px content; outer wrappers stay full-bleed
   (width:100% / max-width:none) so backgrounds extend to viewport edge. */
export function MarketingLayout(): JSX.Element {
  return <Outlet />;
}
