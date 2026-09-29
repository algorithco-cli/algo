import {
  createColumnHelper,
  flexRender,
  getCoreRowModel,
  useReactTable,
} from "@tanstack/react-table";
import * as React from "react";
import { DecisionBadge } from "./DecisionBadge";
import type { DashEntry } from "./entry";
import {
  fmtLatency,
  fmtTs,
  sourceLabel,
  toolKindLabel,
  whyLine,
} from "./entry";

const col = createColumnHelper<DashEntry>();

const baseColumns = [
  col.accessor("timestamp", {
    header: "Time",
    cell: (info) => (
      <span className="dash-mono muted">{fmtTs(info.getValue())}</span>
    ),
  }),
  col.accessor("tool_kind", {
    header: "Tool",
    cell: (info) => (
      <span className="dash-tool">{toolKindLabel(info.getValue())}</span>
    ),
  }),
  col.accessor("action", {
    header: "Action",
    cell: (info) => <DecisionBadge action={info.getValue()} />,
  }),
  col.accessor("redacted_payload", {
    header: "Payload (redacted)",
    cell: (info) => (
      <span className="dash-mono dash-clamp" title={info.getValue()}>
        {info.getValue()}
      </span>
    ),
  }),
  col.accessor("reason", {
    header: "Reason",
    cell: (info) => (
      <span className="dash-clamp">{info.getValue() || "—"}</span>
    ),
  }),
  col.accessor("confidence", {
    header: "Conf.",
    cell: (info) => (
      <span className="dash-mono">{info.getValue().toFixed(2)}</span>
    ),
  }),
  col.accessor("source", {
    header: "Source",
    cell: (info) => (
      <span className="dash-mono muted">{sourceLabel(info.getValue())}</span>
    ),
  }),
  col.accessor("latency_ms", {
    header: "Latency",
    cell: (info) => (
      <span className="dash-mono">{fmtLatency(info.getValue())}</span>
    ),
  }),
];

export function HistoryTable({
  data,
  isLoading,
  demo,
}: {
  data: DashEntry[];
  isLoading?: boolean;
  demo?: boolean;
}): JSX.Element {
  const [expandedId, setExpandedId] = React.useState<string | null>(null);
  const columns = React.useMemo(
    () => [
      ...baseColumns,
      col.accessor("trace_id", {
        header: "Why",
        cell: (info) => {
          const entry = info.row.original;
          const open = expandedId === entry.trace_id;
          return (
            <button
              type="button"
              onClick={() => setExpandedId(open ? null : entry.trace_id)}
              className="dash-why-link"
              aria-expanded={open}
              title={`algo why ${entry.trace_id} — action+reason+confidence+source+latency`}
            >
              why:{entry.trace_id.slice(0, 8)}
            </button>
          );
        },
      }),
    ],
    [expandedId],
  );
  const table = useReactTable({
    data,
    columns,
    getCoreRowModel: getCoreRowModel(),
  });

  if (isLoading) {
    return (
      <div className="card dash-empty" aria-live="polite">
        Loading audit history…
      </div>
    );
  }

  if (data.length === 0) {
    return (
      <div className="card dash-empty">
        No audit entries yet. Run <code>algo log</code> locally or wait for
        daemon events.
      </div>
    );
  }

  const rows = table.getRowModel().rows;
  return (
    <div className="card dash-table-card">
      {demo ? (
        <p className="dash-demo-note">
          <span className="badge badge-ask">demo data</span>{" "}
          <span className="muted small">
            No backend reachable — static redacted fixtures. Sign in with a
            running backend for live history.
          </span>
        </p>
      ) : null}
      <div className="dash-table-scroll">
        <table className="dash-table">
          <thead>
            {table.getHeaderGroups().map((hg) => (
              <tr key={hg.id}>
                {hg.headers.map((h) => (
                  <th key={h.id} className="dash-th">
                    {h.isPlaceholder
                      ? null
                      : flexRender(h.column.columnDef.header, h.getContext())}
                  </th>
                ))}
              </tr>
            ))}
          </thead>
          <tbody>
            {rows.map((row) => {
              const entry = row.original;
              const open = expandedId === entry.trace_id;
              return (
                <React.Fragment key={row.id}>
                  <tr className="dash-tr">
                    {row.getVisibleCells().map((cell) => (
                      <td key={cell.id} className="dash-td">
                        {flexRender(
                          cell.column.columnDef.cell,
                          cell.getContext(),
                        )}
                      </td>
                    ))}
                  </tr>
                  {open ? (
                    <tr className="dash-tr dash-why-row" key={`${row.id}-why`}>
                      <td colSpan={9} className="dash-td">
                        <div className="dash-why">
                          <code>algo why --trace {entry.trace_id}</code>
                          <span className="muted">{whyLine(entry)}</span>
                          {entry.policy_version ? (
                            <span className="muted small">
                              policy {entry.policy_version}
                            </span>
                          ) : null}
                        </div>
                      </td>
                    </tr>
                  ) : null}
                </React.Fragment>
              );
            })}
          </tbody>
        </table>
      </div>
      <div className="dash-table-foot muted small">
        <span>
          {data.length} decision(s) · <code>algo why --trace &lt;id&gt;</code>{" "}
          shows action+reason+confidence+source+latency
        </span>
        <span>
          Run <code>algo log --show-egress</code> to inspect what would leave
          the machine.
        </span>
      </div>
    </div>
  );
}
