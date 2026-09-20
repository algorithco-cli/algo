import * as React from "react";
import { Action } from "../lib/api";
import type { AuditEntry } from "../lib/api";
import { fetchAuditHistoryWithFallback } from "../lib/api";
import { ActionDot } from "./HistoryTable";
import { fmtLatency, fmtTs } from "../lib/utils";

/**
 * LiveFeed — SSE EventSource to /v1/audit/stream with polling fallback.
 * Shares query with TUI/status (same /v1/audit). Static SPA falls back to polling.
 * Fail-safe: SSE error → poll every 5s; never swallows “ask” semantics.
 */

const SSE_URL = "/v1/audit/stream";
const POLL_MS = 5000;

export function LiveFeed({ onNewEntry }: { onNewEntry?: (e: AuditEntry) => void }): JSX.Element {
  const [events, setEvents] = React.useState<AuditEntry[]>([]);
  const [status, setStatus] = React.useState<"sse" | "polling" | "idle">("idle");
  const [error, setError] = React.useState<string | null>(null);

  const appendIfNew = React.useCallback(
    (incoming: AuditEntry) => {
      setEvents((prev) => {
        if (prev.some((p) => p.trace_id === incoming.trace_id)) return prev;
        const next = [incoming, ...prev].slice(0, 20);
        return next;
      });
      onNewEntry?.(incoming);
    },
    [onNewEntry],
  );

  React.useEffect(() => {
    let es: EventSource | null = null;
    let pollTimer: number | null = null;
    let cancelled = false;

    function startPolling(reason: string) {
      if (cancelled) return;
      setStatus("polling");
      setError(reason);
      async function poll() {
        try {
          const data = await fetchAuditHistoryWithFallback({ limit: 1 });
          if (data[0] && !cancelled) appendIfNew(data[0]);
        } catch (e) {
          if (!cancelled) setError(String(e));
        }
      }
      poll();
      pollTimer = window.setInterval(poll, POLL_MS);
    }

    // Try SSE — fail quickly to polling if not available (static build / no daemon)
    try {
      if (typeof window !== "undefined" && "EventSource" in window) {
        setStatus("sse");
        es = new EventSource(SSE_URL);
        es.onopen = () => {
          setStatus("sse");
          setError(null);
        };
        es.onmessage = (ev) => {
          try {
            const parsed = JSON.parse(ev.data) as AuditEntry;
            if (parsed?.trace_id) appendIfNew(parsed);
          } catch {
            // parse fail → ask semantics: keep prior state, log
            setError("SSE parse error — fail-safe: ignoring malformed event (ask)");
          }
        };
        es.onerror = () => {
          // EventSource errors are retry-oriented; after 2s we fall back to polling
          // to keep static build usable without daemon.
          if (es) {
            try {
              es.close();
            } catch {
              // ignore
            }
          }
          if (!cancelled && pollTimer === null) {
            startPolling("SSE unavailable — fallback to polling (static build / no daemon)");
          }
        };
        // If SSE doesn't open within 1.5s, assume static build and poll
        const fallbackTimeout = window.setTimeout(() => {
          if (es && es.readyState !== EventSource.OPEN && pollTimer === null) {
            try {
              es.close();
            } catch {
              // ignore
            }
            startPolling("SSE not reachable — polling /v1/audit");
          }
        }, 1500);
        return () => {
          window.clearTimeout(fallbackTimeout);
        };
      } else {
        startPolling("EventSource not supported — polling");
      }
    } catch (e) {
      startPolling(String(e));
    }

    return () => {
      cancelled = true;
      if (es) {
        try {
          es.close();
        } catch {
          // ignore
        }
      }
      if (pollTimer !== null) window.clearInterval(pollTimer);
    };
  }, [appendIfNew]);

  return (
    <div className="rounded-xl border" style={{ borderColor: "var(--ag-border)", background: "var(--ag-surface)" }}>
      <div className="flex items-center justify-between border-b px-4 py-3" style={{ borderColor: "var(--ag-border)" }}>
        <h3 className="text-sm font-semibold">Live feed</h3>
        <div className="flex items-center gap-2 text-xs">
          <span
            className="inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1"
            style={{
              borderColor: status === "sse" ? "var(--ag-allow)" : "var(--ag-border)",
              color: status === "sse" ? "var(--ag-allow)" : "var(--ag-text-muted)",
              background: status === "sse" ? "color-mix(in srgb, var(--ag-allow) 10%, transparent)" : "transparent",
            }}
            title={status === "sse" ? "SSE connected to /v1/audit/stream" : "Polling /v1/audit"}
          >
            <span className="h-2 w-2 rounded-full" style={{ background: status === "sse" ? "var(--ag-allow)" : "var(--ag-ask)" }} />
            {status === "sse" ? "live" : status === "polling" ? "polling" : "idle"}
          </span>
          <span className="hidden sm:inline" style={{ color: "var(--ag-text-muted)" }}>
            SSE {SSE_URL} → fallback poll {POLL_MS / 1000}s
          </span>
        </div>
      </div>

      {error && (
        <div className="border-b px-4 py-2 text-xs" style={{ borderColor: "var(--ag-border)", color: "var(--ag-text-muted)", background: "var(--ag-bg)" }}>
          {error}
        </div>
      )}

      {events.length === 0 ? (
        <div className="p-6 text-center text-sm" style={{ color: "var(--ag-text-muted)" }}>
          No live events yet — waiting for <code className="rounded bg-[var(--ag-bg)] px-1 py-0.5">/v1/audit/stream</code>. Static build shows
          polling preview; live daemon will push decisions here.
        </div>
      ) : (
        <ul className="max-h-[320px] overflow-y-auto divide-y" style={{ borderColor: "var(--ag-border)" }}>
          {events.map((e) => (
            <li key={e.trace_id} className="flex items-start gap-3 px-4 py-2.5">
              <span className="mt-1.5">
                <ActionDot action={e.action} />
              </span>
              <div className="min-w-0 flex-1">
                <div className="flex flex-wrap items-center gap-2">
                  <span
                    className="inline-flex rounded-full px-2 py-0.5 text-xs font-semibold"
                    style={{
                      background:
                        e.action === Action.ACTION_ALLOW
                          ? "var(--ag-allow)"
                          : e.action === Action.ACTION_DENY
                            ? "var(--ag-deny)"
                            : "var(--ag-ask)",
                      color: e.action === Action.ACTION_ASK ? "black" : "white",
                    }}
                  >
                    {e.action === Action.ACTION_ALLOW ? "allow" : e.action === Action.ACTION_DENY ? "deny" : "ask"}
                  </span>
                  <span className="truncate font-mono text-xs" title={e.redacted_payload}>
                    {e.redacted_payload}
                  </span>
                </div>
                <div className="mt-1 flex flex-wrap gap-2 text-xs" style={{ color: "var(--ag-text-muted)" }}>
                  <span>{fmtTs(e.timestamp)}</span>
                  <span>·</span>
                  <span>{e.reason}</span>
                  <span>·</span>
                  <span>{fmtLatency(e.latency_ms)}</span>
                  <span>·</span>
                  <a
                    href="#"
                    onClick={(ev) => {
                      ev.preventDefault();
                      alert(`algo why ${e.trace_id}\n${e.reason} · conf ${e.confidence.toFixed(2)} · latency ${e.latency_ms}ms`);
                    }}
                    className="underline decoration-dotted"
                    style={{ color: "var(--ag-brand)" }}
                  >
                    why:{e.trace_id.slice(0, 8)}
                  </a>
                </div>
              </div>
            </li>
          ))}
        </ul>
      )}

      <div className="border-t px-4 py-2 text-xs" style={{ borderColor: "var(--ag-border)", color: "var(--ag-text-muted)" }}>
        Shares query with TUI/status — same <code>/v1/audit</code> source. View full history below.
      </div>
    </div>
  );
}
