import { useQuery } from "@tanstack/react-query";
import * as React from "react";
import "uplot/dist/uPlot.min.css";
import type uPlot from "uplot";
import { pct } from "../../components/dashboard/entry";
import {
  mockPerProject,
  mockPerUser,
  mockStats,
  mockTimeSeries,
  savedMinutes,
} from "../../components/dashboard/mock";
import { guardKeys, guardOrgId, guardToken, queryStats } from "../../lib/guard";

type UPlotCtor = typeof uPlot;

function chartSeries(
  buckets:
    | { bucket_start: string; allow: number; ask: number; deny: number }[]
    | undefined,
): { xs: number[]; allow: number[]; ask: number[]; deny: number[] } {
  if (buckets && buckets.length > 0) {
    return {
      xs: buckets.map((b) => Date.parse(b.bucket_start) / 1000),
      allow: buckets.map((b) => b.allow),
      ask: buckets.map((b) => b.ask),
      deny: buckets.map((b) => b.deny),
    };
  }
  return {
    xs: mockTimeSeries.map((d) => d.ts),
    allow: mockTimeSeries.map((d) => d.allow),
    ask: mockTimeSeries.map((d) => d.ask),
    deny: mockTimeSeries.map((d) => d.deny),
  };
}

/** Stats: KPIs + uPlot trend + per-user/project, live with demo fallback. */
export default function DashboardStats(): JSX.Element {
  const token = guardToken();
  const orgId = guardOrgId();
  const q = useQuery({
    queryKey: guardKeys.stats({
      org_id: orgId || undefined,
      granularity: "day",
      limit: 30,
      top_n: 10,
    }),
    queryFn: () =>
      queryStats(
        {
          org_id: orgId || undefined,
          granularity: "day",
          limit: 30,
          top_n: 10,
        },
        token,
      ),
    enabled: !!token,
    retry: false,
  });
  const demo = !q.data;
  const total = q.data?.total ?? mockStats.total;
  const allow = q.data?.allow ?? mockStats.allow;
  const ask = q.data?.ask ?? mockStats.ask;
  const deny = q.data?.deny ?? mockStats.deny;
  const avg = q.data?.avg_latency_ms ?? mockStats.avg_latency_ms;
  const savedH = (savedMinutes(allow, ask, deny) / 60).toFixed(1);

  const series = React.useMemo(() => chartSeries(q.data?.buckets), [q.data]);
  const chartLive = !!q.data?.buckets?.length;
  const perUser = q.data?.per_user;
  const chartRef = React.useRef<HTMLDivElement>(null);
  const plotRef = React.useRef<uPlot | null>(null);

  React.useEffect(() => {
    let cancelled = false;
    async function mount() {
      if (!chartRef.current) return;
      const mod = (await import("uplot")) as unknown as {
        default?: UPlotCtor;
      } & UPlotCtor;
      const Ctor = mod.default ?? (mod as unknown as UPlotCtor);
      if (cancelled || !chartRef.current) return;
      const cs = getComputedStyle(document.documentElement);
      const css = (v: string): string =>
        cs.getPropertyValue(v).trim() || `var(${v})`;
      const opts = {
        width: chartRef.current.clientWidth,
        height: 180,
        scales: { x: { time: true } },
        axes: [
          {
            stroke: css("--ag-text-muted"),
            grid: { show: true, stroke: css("--ag-border"), width: 1 },
          },
          { stroke: css("--ag-text-muted"), grid: { show: true } },
        ],
        series: [
          {},
          {
            label: "allow",
            stroke: css("--ag-allow"),
            width: 2,
            fill: `${css("--ag-allow")}20`,
          },
          {
            label: "ask",
            stroke: css("--ag-ask"),
            width: 2,
            fill: `${css("--ag-ask")}20`,
          },
          {
            label: "deny",
            stroke: css("--ag-deny"),
            width: 2,
            fill: `${css("--ag-deny")}20`,
          },
        ],
      } as unknown as ConstructorParameters<UPlotCtor>[0];
      try {
        plotRef.current?.destroy();
      } catch {
        /* ignore */
      }
      plotRef.current = new Ctor(
        opts,
        [series.xs, series.allow, series.ask, series.deny],
        chartRef.current,
      );
      const ro = new ResizeObserver(() => {
        if (chartRef.current && plotRef.current) {
          plotRef.current.setSize({
            width: chartRef.current.clientWidth,
            height: 180,
          });
        }
      });
      if (chartRef.current) ro.observe(chartRef.current);
      return () => ro.disconnect();
    }
    const cleanup = mount();
    return () => {
      cancelled = true;
      void cleanup.then((fn) => fn?.());
      try {
        plotRef.current?.destroy();
      } catch {
        /* ignore */
      }
      plotRef.current = null;
    };
  }, [series.xs, series.allow, series.ask, series.deny]);

  return (
    <section aria-label="Stats">
      <h1 className="dash-title">Stats</h1>
      <p className="muted">
        Per-user / per-project · time savings · blocked/asked breakdown.{" "}
        {demo ? (
          <>
            <span className="badge badge-ask">demo data</span> Sign in with a
            running backend for live aggregates.
          </>
        ) : (
          "Live aggregates from redacted audit."
        )}
      </p>
      <div className="dash-stack">
        <div className="dash-grid-4">
          <div className="card">
            <h2 className="dash-card-title">Total decisions</h2>
            <p className="dash-kpi">{total}</p>
            <p className="muted small">
              avg latency {Math.round(avg)}ms · L0 target &lt;3ms (CI gate)
            </p>
          </div>
          {(
            [
              ["Allow", allow, "--ag-allow"],
              ["Ask", ask, "--ag-ask"],
              ["Deny", deny, "--ag-deny"],
            ] as const
          ).map(([label, n, color]) => (
            <div className="card" key={label}>
              <h2 className="dash-card-title">{label}</h2>
              <p className="dash-kpi" style={{ color: `var(${color})` }}>
                {n} <span className="muted small">{pct(n, total)}</span>
              </p>
              <div className="dash-bar">
                <div
                  className="dash-bar-fill"
                  style={{
                    width: `${total ? (n / total) * 100 : 0}%`,
                    background: `var(${color})`,
                  }}
                />
              </div>
            </div>
          ))}
        </div>

        <div className="dash-grid-2">
          <div className="card">
            <h2 className="dash-card-title">Trend</h2>
            <div ref={chartRef} className="dash-chart" />
            <p className="muted small">
              Source: <code>/v1/stats</code> QueryStats buckets
              {chartLive ? "" : " (demo series — static fallback)"}.
            </p>
          </div>
          <div className="card">
            <h2 className="dash-card-title">Time savings</h2>
            <p className="dash-kpi">{savedH}h</p>
            <p className="muted small">
              Est. review time saved (~30s per ask/deny + 3s per cached allow ·{" "}
              {ask + deny} interventions)
            </p>
            <p className="muted small">
              Static estimate — no vendor claim without measurement link.
            </p>
          </div>
        </div>

        <div className="dash-grid-2">
          <div className="card">
            <h2 className="dash-card-title">Per-user stats</h2>
            {perUser && perUser.length > 0 ? (
              <table className="dash-table">
                <thead>
                  <tr>
                    <th className="dash-th">User</th>
                    <th className="dash-th dash-num">Allow</th>
                    <th className="dash-th dash-num">Ask</th>
                    <th className="dash-th dash-num">Deny</th>
                  </tr>
                </thead>
                <tbody>
                  {perUser.map((r) => (
                    <tr className="dash-tr" key={r.user_id}>
                      <td className="dash-td dash-mono">{r.user_id}</td>
                      <td className="dash-td dash-num">{r.allow}</td>
                      <td className="dash-td dash-num">{r.ask}</td>
                      <td className="dash-td dash-num">{r.deny}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            ) : !demo ? (
              <p className="muted small">
                No user attribution yet — agents are not sending user_id.
              </p>
            ) : (
              <table className="dash-table">
                <thead>
                  <tr>
                    <th className="dash-th">User</th>
                    <th className="dash-th dash-num">Allow</th>
                    <th className="dash-th dash-num">Ask</th>
                    <th className="dash-th dash-num">Deny</th>
                    <th className="dash-th dash-num">Saved (min)</th>
                  </tr>
                </thead>
                <tbody>
                  {mockPerUser.map((r) => (
                    <tr className="dash-tr" key={r.user}>
                      <td className="dash-td dash-mono">{r.user}</td>
                      <td className="dash-td dash-num">{r.allow}</td>
                      <td className="dash-td dash-num">{r.ask}</td>
                      <td className="dash-td dash-num">{r.deny}</td>
                      <td className="dash-td dash-num">{r.saved_min}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
          <div className="card">
            <h2 className="dash-card-title">Per-project stats</h2>
            {!demo ? (
              <p className="muted small">
                No project attribution yet — Phase 2 follow-up.
              </p>
            ) : (
              <table className="dash-table">
                <thead>
                  <tr>
                    <th className="dash-th">Project</th>
                    <th className="dash-th dash-num">Allow</th>
                    <th className="dash-th dash-num">Ask</th>
                    <th className="dash-th dash-num">Deny</th>
                    <th className="dash-th dash-num">Saved (min)</th>
                  </tr>
                </thead>
                <tbody>
                  {mockPerProject.map((r) => (
                    <tr className="dash-tr" key={r.project}>
                      <td className="dash-td dash-mono">{r.project}</td>
                      <td className="dash-td dash-num">{r.allow}</td>
                      <td className="dash-td dash-num">{r.ask}</td>
                      <td className="dash-td dash-num">{r.deny}</td>
                      <td className="dash-td dash-num">{r.saved_min}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
        </div>

        <div className="card">
          <h2 className="dash-card-title">Security findings</h2>
          <p className="muted small">
            Placeholder — P4 fills (scanner findings + code-review signals). No
            mock findings emitted in MVP. Findings will reuse allow/ask/deny
            badge semantics and link the correlated <code>algo why</code> trace.
          </p>
        </div>
      </div>
    </section>
  );
}
