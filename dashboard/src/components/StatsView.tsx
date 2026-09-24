import { useQuery } from "@tanstack/react-query";
import * as React from "react";
import { queryStatsWithFallback } from "../lib/fallback";
import { mockPerProject, mockPerUser, mockTimeSeries } from "../lib/mock";
import { pct } from "../lib/utils";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "./ui/card";

// uPlot is bundled static; we init via effect to avoid SSR issues
import type UPlot from "uplot";
type UPlotCtor = typeof UPlot;

export function StatsView(): JSX.Element {
  const statsQ = useQuery({
    queryKey: ["stats"],
    queryFn: () => queryStatsWithFallback({}),
  });

  const total = statsQ.data?.total ?? 0;
  const allow = statsQ.data?.allow ?? 0;
  const ask = statsQ.data?.ask ?? 0;
  const deny = statsQ.data?.deny ?? 0;
  const avg = statsQ.data?.avg_latency_ms ?? 0;

  // Cost/time savings (static estimate — no vendor claim without measurement link per docs)
  // Assumption documented in UI: 30s review saved per blocked/asked intervention; $ placeholder uses $0 cost model for static build.
  const savedInterventions = ask + deny;
  const savedMinutes = savedInterventions * 0.5 + allow * 0.05; // 30s per ask/deny, 3s per allow cache hit
  const savedHours = (savedMinutes / 60).toFixed(1);

  const chartRef = React.useRef<HTMLDivElement>(null);
  const uplotRef = React.useRef<UPlot | null>(null);

  React.useEffect(() => {
    let cancelled = false;
    async function mountUplot() {
      if (!chartRef.current) return;
      // Lazy import to keep static chunk splitting clean — uplot has no default export in types, use namespace
      const uPlotMod = await import("uplot");
      const UPlot =
        (uPlotMod as unknown as { default: UPlotCtor }).default ??
        (uPlotMod as unknown as UPlotCtor as unknown as UPlotCtor);
      // Import uPlot CSS via JS injection fallback — static build includes dist/uPlot.min.css if available
      if (cancelled || !chartRef.current) return;
      const xs = mockTimeSeries.map((d) => d.ts);
      const allowY = mockTimeSeries.map((d) => d.allow);
      const askY = mockTimeSeries.map((d) => d.ask);
      const denyY = mockTimeSeries.map((d) => d.deny);
      const cs = getComputedStyle(document.documentElement);
      const allowColor =
        cs.getPropertyValue("--ag-allow").trim() || "var(--ag-allow)";
      const askColor =
        cs.getPropertyValue("--ag-ask").trim() || "var(--ag-ask)";
      const denyColor =
        cs.getPropertyValue("--ag-deny").trim() || "var(--ag-deny)";
      const muted =
        cs.getPropertyValue("--ag-text-muted").trim() || "var(--ag-text-muted)";
      const border =
        cs.getPropertyValue("--ag-border").trim() || "var(--ag-border)";
      const opts = {
        width: chartRef.current.clientWidth,
        height: 180,
        scales: { x: { time: true } },
        axes: [
          { stroke: muted, grid: { show: true, stroke: border, width: 1 } },
          { stroke: muted, grid: { show: true } },
        ],
        series: [
          {},
          {
            label: "allow",
            stroke: allowColor,
            width: 2,
            fill: `${allowColor}20`,
          },
          { label: "ask", stroke: askColor, width: 2, fill: `${askColor}20` },
          {
            label: "deny",
            stroke: denyColor,
            width: 2,
            fill: `${denyColor}20`,
          },
        ],
      } as unknown as ConstructorParameters<UPlotCtor>[0];
      if (uplotRef.current) {
        try {
          uplotRef.current.destroy();
        } catch {
          // ignore
        }
      }
      uplotRef.current = new UPlot(
        opts,
        [xs, allowY, askY, denyY],
        chartRef.current,
      );
      const ro = new ResizeObserver(() => {
        if (!chartRef.current || !uplotRef.current) return;
        uplotRef.current.setSize({
          width: chartRef.current.clientWidth,
          height: 180,
        });
      });
      ro.observe(chartRef.current);
      return () => ro.disconnect();
    }
    mountUplot();
    return () => {
      cancelled = true;
      if (uplotRef.current) {
        try {
          uplotRef.current.destroy();
        } catch {
          // ignore
        }
        uplotRef.current = null;
      }
    };
  }, []);

  return (
    <div className="grid gap-6">
      {/* Top KPI cards */}
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Total decisions</CardDescription>
            <CardTitle className="text-2xl">{total}</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-xs" style={{ color: "var(--ag-text-muted)" }}>
              avg latency {avg}ms · p50/L0 target &lt;3ms (CI gate)
            </p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>
              <span className="inline-flex items-center gap-1.5">
                <span
                  className="h-2 w-2 rounded-full"
                  style={{ background: "var(--ag-allow)" }}
                />{" "}
                Allow
              </span>
            </CardDescription>
            <CardTitle
              className="text-2xl"
              style={{ color: "var(--ag-allow)" }}
            >
              {allow}{" "}
              <span
                className="text-sm font-normal"
                style={{ color: "var(--ag-text-muted)" }}
              >
                {pct(allow, total)}
              </span>
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div
              className="h-2 w-full rounded-full"
              style={{ background: "var(--ag-bg)" }}
            >
              <div
                className="h-2 rounded-full"
                style={{
                  width: `${total ? (allow / total) * 100 : 0}%`,
                  background: "var(--ag-allow)",
                }}
              />
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>
              <span className="inline-flex items-center gap-1.5">
                <span
                  className="h-2 w-2 rounded-full"
                  style={{ background: "var(--ag-ask)" }}
                />{" "}
                Ask
              </span>
            </CardDescription>
            <CardTitle className="text-2xl" style={{ color: "var(--ag-ask)" }}>
              {ask}{" "}
              <span
                className="text-sm font-normal"
                style={{ color: "var(--ag-text-muted)" }}
              >
                {pct(ask, total)}
              </span>
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div
              className="h-2 w-full rounded-full"
              style={{ background: "var(--ag-bg)" }}
            >
              <div
                className="h-2 rounded-full"
                style={{
                  width: `${total ? (ask / total) * 100 : 0}%`,
                  background: "var(--ag-ask)",
                }}
              />
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>
              <span className="inline-flex items-center gap-1.5">
                <span
                  className="h-2 w-2 rounded-full"
                  style={{ background: "var(--ag-deny)" }}
                />{" "}
                Deny
              </span>
            </CardDescription>
            <CardTitle className="text-2xl" style={{ color: "var(--ag-deny)" }}>
              {deny}{" "}
              <span
                className="text-sm font-normal"
                style={{ color: "var(--ag-text-muted)" }}
              >
                {pct(deny, total)}
              </span>
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div
              className="h-2 w-full rounded-full"
              style={{ background: "var(--ag-bg)" }}
            >
              <div
                className="h-2 rounded-full"
                style={{
                  width: `${total ? (deny / total) * 100 : 0}%`,
                  background: "var(--ag-deny)",
                }}
              />
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Cost / time savings */}
      <div className="grid gap-4 lg:grid-cols-3">
        <Card className="lg:col-span-2">
          <CardHeader>
            <CardTitle>14-day trend</CardTitle>
            <CardDescription>
              Decisions per day — allow/ask/deny (uPlot, static mock when no
              backend)
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div
              ref={chartRef}
              className="w-full overflow-hidden rounded border"
              style={{ borderColor: "var(--ag-border)" }}
            />
            <p
              className="mt-2 text-xs"
              style={{ color: "var(--ag-text-muted)" }}
            >
              Source: <code>/v1/stats?from=&amp;to=</code> (QueryStats). This
              chart uses mockTimeSeries fallback in static build.
            </p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Time &amp; cost savings</CardTitle>
            <CardDescription>
              Static estimate — no vendor claim without measurement link
            </CardDescription>
          </CardHeader>
          <CardContent className="grid gap-4">
            <div
              className="rounded-lg border p-4"
              style={{
                borderColor: "var(--ag-border)",
                background: "var(--ag-bg)",
              }}
            >
              <div
                className="text-xs"
                style={{ color: "var(--ag-text-muted)" }}
              >
                Est. review time saved
              </div>
              <div className="text-2xl font-semibold">{savedHours}h</div>
              <div
                className="text-xs"
                style={{ color: "var(--ag-text-muted)" }}
              >
                ~30s per ask/deny + 3s per cached allow · {savedInterventions}{" "}
                interventions
              </div>
            </div>
            <div
              className="rounded-lg border p-4"
              style={{
                borderColor: "var(--ag-border)",
                background: "var(--ag-bg)",
              }}
            >
              <div
                className="text-xs"
                style={{ color: "var(--ag-text-muted)" }}
              >
                Blocked/asked breakdown
              </div>
              <div className="mt-2 grid gap-1 text-sm">
                <div className="flex justify-between">
                  <span>
                    <span
                      className="inline-block h-2 w-2 rounded-full"
                      style={{ background: "var(--ag-deny)" }}
                    />{" "}
                    Blocked
                  </span>
                  <span className="font-mono">{deny}</span>
                </div>
                <div className="flex justify-between">
                  <span>
                    <span
                      className="inline-block h-2 w-2 rounded-full"
                      style={{ background: "var(--ag-ask)" }}
                    />{" "}
                    Needs review
                  </span>
                  <span className="font-mono">{ask}</span>
                </div>
                <div className="flex justify-between">
                  <span>
                    <span
                      className="inline-block h-2 w-2 rounded-full"
                      style={{ background: "var(--ag-allow)" }}
                    />{" "}
                    Auto-allowed
                  </span>
                  <span className="font-mono">{allow}</span>
                </div>
              </div>
            </div>
            <p className="text-xs" style={{ color: "var(--ag-text-muted)" }}>
              Savings math is an in-UI placeholder for the MVP static build.
              Prod will use backend-measured stats (see P3 backend QueryStats).
              No cost claim without eval link per AGENTS.md §8.
            </p>
          </CardContent>
        </Card>
      </div>

      {/* Per-user / per-project */}
      <div className="grid gap-4 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Per-user stats</CardTitle>
            <CardDescription>
              Static mock — prod will scope by org_id via /v1/stats
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr
                    className="border-b text-xs"
                    style={{
                      borderColor: "var(--ag-border)",
                      color: "var(--ag-text-muted)",
                    }}
                  >
                    <th className="px-2 py-2 text-left">User</th>
                    <th className="px-2 py-2 text-right">Allow</th>
                    <th className="px-2 py-2 text-right">Ask</th>
                    <th className="px-2 py-2 text-right">Deny</th>
                    <th className="px-2 py-2 text-right">Saved (min)</th>
                  </tr>
                </thead>
                <tbody>
                  {mockPerUser.map((r) => (
                    <tr
                      key={r.user}
                      className="border-b last:border-0"
                      style={{ borderColor: "var(--ag-border)" }}
                    >
                      <td className="px-2 py-2 font-mono text-xs">{r.user}</td>
                      <td
                        className="px-2 py-2 text-right font-mono"
                        style={{ color: "var(--ag-allow)" }}
                      >
                        {r.allow}
                      </td>
                      <td
                        className="px-2 py-2 text-right font-mono"
                        style={{ color: "var(--ag-ask)" }}
                      >
                        {r.ask}
                      </td>
                      <td
                        className="px-2 py-2 text-right font-mono"
                        style={{ color: "var(--ag-deny)" }}
                      >
                        {r.deny}
                      </td>
                      <td className="px-2 py-2 text-right font-mono">
                        {r.saved_min}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Per-project stats</CardTitle>
            <CardDescription>
              Static mock — prod will use daemon-reported project fingerprint
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr
                    className="border-b text-xs"
                    style={{
                      borderColor: "var(--ag-border)",
                      color: "var(--ag-text-muted)",
                    }}
                  >
                    <th className="px-2 py-2 text-left">Project</th>
                    <th className="px-2 py-2 text-right">Allow</th>
                    <th className="px-2 py-2 text-right">Ask</th>
                    <th className="px-2 py-2 text-right">Deny</th>
                    <th className="px-2 py-2 text-right">Saved (min)</th>
                  </tr>
                </thead>
                <tbody>
                  {mockPerProject.map((r) => (
                    <tr
                      key={r.project}
                      className="border-b last:border-0"
                      style={{ borderColor: "var(--ag-border)" }}
                    >
                      <td className="px-2 py-2 font-mono text-xs">
                        {r.project}
                      </td>
                      <td
                        className="px-2 py-2 text-right font-mono"
                        style={{ color: "var(--ag-allow)" }}
                      >
                        {r.allow}
                      </td>
                      <td
                        className="px-2 py-2 text-right font-mono"
                        style={{ color: "var(--ag-ask)" }}
                      >
                        {r.ask}
                      </td>
                      <td
                        className="px-2 py-2 text-right font-mono"
                        style={{ color: "var(--ag-deny)" }}
                      >
                        {r.deny}
                      </td>
                      <td className="px-2 py-2 text-right font-mono">
                        {r.saved_min}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Security findings placeholder (P4) */}
      <Card className="border-dashed">
        <CardHeader>
          <CardTitle>Security findings</CardTitle>
          <CardDescription>
            Placeholder — P4 fills (scanner findings + code-review signals). No
            mock findings emitted in MVP.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div
            className="rounded-lg border border-dashed p-8 text-center"
            style={{
              borderColor: "var(--ag-border)",
              background: "var(--ag-bg)",
            }}
          >
            <p className="text-sm font-medium">
              No scanner findings in this build
            </p>
            <p
              className="mx-auto mt-1 max-w-[60ch] text-xs"
              style={{ color: "var(--ag-text-muted)" }}
            >
              Phase 4 will add repo-scan + scanner integration here. Findings
              will reuse the same allow/ask/deny badge semantics and link to the
              correlated <code>algo why</code> trace. Static SPA shows this
              placeholder until the backend audit stream provides findings.
            </p>
            <p className="mt-3 text-xs">
              <a
                href="https://github.com/anomalyco/opencode"
                className="underline decoration-dotted"
                style={{ color: "var(--ag-brand)" }}
              >
                Plans: phase-4-scanner-questions.md
              </a>
            </p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
