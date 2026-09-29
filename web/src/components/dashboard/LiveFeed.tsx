import { useQueryClient } from "@tanstack/react-query";
import * as React from "react";
import { fetchAuditStream, guardOrgId, guardToken } from "../../lib/guard";
import { ActionDot, DecisionBadge } from "./DecisionBadge";
import type { DashEntry } from "./entry";
import { fmtLatency, fmtTs, recordToEntry, whyLine } from "./entry";

/**
 * LiveFeed — authed tail over GET /v1/audit/stream (bounded bursts).
 * EventSource carries no auth headers, so this polls fetchAuditStream every
 * 5s with the session token and resumes via burst cursor. Signed out → static
 * note (tables below still show demo data). Fail-safe: fetch errors surface
 * as polling status, never silent allow.
 */

const POLL_MS = 5000;
const MAX_EVENTS = 20;

export function LiveFeed(): JSX.Element {
  const qc = useQueryClient();
  const token = guardToken();
  const orgId = guardOrgId();
  const [events, setEvents] = React.useState<DashEntry[]>([]);
  const [live, setLive] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  const [expandedId, setExpandedId] = React.useState<string | null>(null);
  const cursorRef = React.useRef("");

  React.useEffect(() => {
    if (!token) return;
    let cancelled = false;
    let timer = 0;
    async function poll() {
      try {
        const burst = await fetchAuditStream(
          {
            org_id: orgId || undefined,
            limit: MAX_EVENTS,
            cursor: cursorRef.current || undefined,
          },
          token,
        );
        if (cancelled) return;
        if (burst.lastId) cursorRef.current = burst.lastId;
        const fresh = burst.records.map(recordToEntry);
        if (fresh.length > 0) {
          setEvents((prev) => {
            const known = new Set(prev.map((p) => p.trace_id));
            const next = [
              ...fresh.filter((f) => !known.has(f.trace_id)),
              ...prev,
            ].slice(0, MAX_EVENTS);
            return next;
          });
          qc.invalidateQueries({ queryKey: ["guard", "audit"] });
        }
        setLive(true);
        setError(null);
      } catch (e) {
        if (!cancelled) {
          setLive(false);
          setError(e instanceof Error ? e.message : "Stream unavailable");
        }
      }
    }
    poll();
    timer = window.setInterval(poll, POLL_MS);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [token, orgId, qc]);

  if (!token) {
    return (
      <div className="card">
        <h2 className="dash-card-title">Live feed</h2>
        <p className="muted small">
          Sign in to stream live decisions from <code>/v1/audit/stream</code>.
          History below shows demo data until then.
        </p>
      </div>
    );
  }

  return (
    <div className="card dash-live">
      <div className="dash-live-head">
        <h2 className="dash-card-title">Live feed</h2>
        <span className={live ? "badge badge-allow" : "badge badge-ask"}>
          {live ? "live" : "polling"}
        </span>
      </div>
      {error ? <p className="muted small dash-live-err">{error}</p> : null}
      {events.length === 0 ? (
        <p className="muted small">
          No live events yet — waiting for <code>/v1/audit/stream</code>.
        </p>
      ) : (
        <ul className="dash-feed">
          {events.map((e) => {
            const open = expandedId === e.trace_id;
            return (
              <li key={e.trace_id} className="dash-feed-row">
                <span className="dash-feed-dot">
                  <ActionDot action={e.action} />
                </span>
                <div className="dash-feed-body">
                  <div className="dash-feed-line">
                    <DecisionBadge action={e.action} />
                    <span
                      className="dash-mono dash-truncate"
                      title={e.redacted_payload}
                    >
                      {e.redacted_payload}
                    </span>
                  </div>
                  <div className="muted small dash-feed-meta">
                    <span>{fmtTs(e.timestamp)}</span>
                    <span>{e.reason || "no reason recorded"}</span>
                    <span>{fmtLatency(e.latency_ms)}</span>
                    <button
                      type="button"
                      onClick={() => setExpandedId(open ? null : e.trace_id)}
                      className="dash-why-link"
                      aria-expanded={open}
                    >
                      why:{e.trace_id.slice(0, 8)}
                    </button>
                  </div>
                  {open ? (
                    <div className="dash-why">
                      <code>algo why --trace {e.trace_id}</code>
                      <span className="muted">{whyLine(e)}</span>
                    </div>
                  ) : null}
                </div>
              </li>
            );
          })}
        </ul>
      )}
      <p className="muted small">
        Same <code>/v1/audit</code> source as history and CLI status.
      </p>
    </div>
  );
}
