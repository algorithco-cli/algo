import * as React from "react";
import {
  createColumnHelper,
  flexRender,
  getCoreRowModel,
  useReactTable,
} from "@tanstack/react-table";
import type { AuditEntry } from "../lib/api";
import { Action } from "../lib/api";
import { DecisionBadge } from "./ui/badge";
import { sourceLabel, toolKindLabel } from "../lib/mock";
import { fmtLatency, fmtTs } from "../lib/utils";

const col = createColumnHelper<AuditEntry>();

const columns = [
  col.accessor("timestamp", {
    header: "Time",
    cell: (info) => (
      <span className="whitespace-nowrap text-xs font-mono" style={{ color: "var(--ag-text-muted)" }}>
        {fmtTs(info.getValue())}
      </span>
    ),
    size: 180,
  }),
  col.accessor("tool_kind", {
    header: "Tool",
    cell: (info) => (
      <span className="inline-flex rounded bg-[var(--ag-bg)] px-1.5 py-0.5 text-xs font-mono border" style={{ borderColor: "var(--ag-border)" }}>
        {toolKindLabel(info.getValue())}
      </span>
    ),
    size: 90,
  }),
  col.accessor("action", {
    header: "Action",
    cell: (info) => <DecisionBadge action={info.getValue()} />,
    size: 90,
  }),
  col.accessor("redacted_payload", {
    header: "Payload (redacted)",
    cell: (info) => (
      <span className="font-mono text-xs line-clamp-2 max-w-[28ch] sm:max-w-[40ch]" title={info.getValue()}>
        {info.getValue()}
      </span>
    ),
  }),
  col.accessor("reason", {
    header: "Reason",
    cell: (info) => <span className="text-sm line-clamp-2 max-w-[30ch]">{info.getValue()}</span>,
  }),
  col.accessor("confidence", {
    header: "Conf.",
    cell: (info) => <span className="font-mono text-xs">{info.getValue().toFixed(2)}</span>,
    size: 70,
  }),
  col.accessor("source", {
    header: "Source",
    cell: (info) => (
      <span className="text-xs font-mono" style={{ color: "var(--ag-text-muted)" }}>
        {sourceLabel(info.getValue())}
      </span>
    ),
    size: 110,
  }),
  col.accessor("latency_ms", {
    header: "Latency",
    cell: (info) => <span className="font-mono text-xs">{fmtLatency(info.getValue())}</span>,
    size: 80,
  }),
  col.accessor("trace_id", {
    header: "Why",
    cell: (info) => (
      <a
        href={`#why-${info.getValue()}`}
        onClick={(e) => {
          e.preventDefault();
          const ev = new CustomEvent("algo:why", { detail: { trace_id: info.getValue() } });
          window.dispatchEvent(ev);
          // Fallback: copy algo why command to clipboard hint
          alert(`algo why --trace ${info.getValue()}\n\nAction + reason + confidence + source + latency are shown in this row.\nRun \`algo why ${info.getValue()}\` in your terminal or inspect \`algo log --trace ${info.getValue()}\`.`);
        }}
        className="text-xs font-mono underline decoration-dotted underline-offset-4"
        style={{ color: "var(--ag-brand)" }}
        title={`algo why ${info.getValue()} — action+reason+confidence+source+latency`}
      >
        why:{info.getValue().slice(0, 8)}
      </a>
    ),
    size: 140,
  }),
];

export function HistoryTable({ data, isLoading }: { data: AuditEntry[]; isLoading?: boolean }): JSX.Element {
  const table = useReactTable({
    data,
    columns,
    getCoreRowModel: getCoreRowModel(),
  });

  if (isLoading) {
    return (
      <div className="rounded-xl border p-8 text-center text-sm" style={{ borderColor: "var(--ag-border)", color: "var(--ag-text-muted)" }}>
        Loading audit history…
      </div>
    );
  }

  if (data.length === 0) {
    return (
      <div className="rounded-xl border p-8 text-center text-sm" style={{ borderColor: "var(--ag-border)", color: "var(--ag-text-muted)" }}>
        No audit entries yet. Run <code className="rounded bg-[var(--ag-bg)] px-1 py-0.5">algo log</code> locally or wait for daemon events.
      </div>
    );
  }

  return (
    <div className="overflow-x-auto rounded-xl border" style={{ borderColor: "var(--ag-border)", background: "var(--ag-surface)" }}>
      <table className="w-full text-sm">
        <thead style={{ background: "var(--ag-bg)", borderBottom: "1px solid var(--ag-border)" }}>
          {table.getHeaderGroups().map((hg) => (
            <tr key={hg.id}>
              {hg.headers.map((h) => (
                <th
                  key={h.id}
                  className="px-3 py-2 text-left text-xs font-semibold tracking-wide"
                  style={{ color: "var(--ag-text-muted)", width: h.getSize() ? `${h.getSize()}px` : undefined }}
                >
                  {h.isPlaceholder ? null : flexRender(h.column.columnDef.header, h.getContext())}
                </th>
              ))}
            </tr>
          ))}
        </thead>
        <tbody>
          {table.getRowModel().rows.map((row) => (
            <tr key={row.id} className="border-t hover:opacity-[0.98]" style={{ borderColor: "var(--ag-border)" }}>
              {row.getVisibleCells().map((cell) => (
                <td key={cell.id} className="px-3 py-2 align-middle">
                  {flexRender(cell.column.columnDef.cell, cell.getContext())}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
      <div className="flex items-center justify-between border-t px-3 py-2 text-xs" style={{ borderColor: "var(--ag-border)", color: "var(--ag-text-muted)" }}>
        <span>
          {data.length} decision(s) · <code>algo why --trace &lt;id&gt;</code> shows action+reason+confidence+source+latency
        </span>
        <a
          href="#"
          onClick={(e) => {
            e.preventDefault();
            alert("Tip: run `algo log --show-egress` to inspect exactly what would leave the machine (redacted payload).");
          }}
          className="underline decoration-dotted"
          style={{ color: "var(--ag-brand)" }}
        >
          show egress
        </a>
      </div>
    </div>
  );
}

// Helper for inline badge-less contextual rendering (used in LiveFeed compact rows)
export function ActionDot({ action }: { action: Action }): JSX.Element {
  const color =
    action === Action.ACTION_ALLOW
      ? "var(--ag-allow)"
      : action === Action.ACTION_DENY
        ? "var(--ag-deny)"
        : "var(--ag-ask)";
  return <span className="inline-block h-2 w-2 rounded-full" style={{ background: color }} aria-hidden />;
}
