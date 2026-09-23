import * as React from "react";

const NODES = ["Hook", "Parse + Redact", "Decide", "Render + Audit"];
const W = 190;
const GAP = 46;

export const PipelineDiagram = React.memo(
  function PipelineDiagram(): JSX.Element {
    const total = NODES.length * W + (NODES.length - 1) * GAP;
    return (
      <svg
        className="pipeline-diagram"
        viewBox={`0 0 ${total} 120`}
        role="img"
        aria-label="Pipeline diagram: Hook flows to Parse plus Redact, then Decide, then Render plus Audit."
      >
        <defs>
          <marker
            id="pipe-arrow"
            viewBox="0 0 10 10"
            refX="8"
            refY="5"
            markerWidth="7"
            markerHeight="7"
            orient="auto-start-reverse"
          >
            <path d="M 0 1 L 9 5 L 0 9 z" className="flow-arrow-head" />
          </marker>
        </defs>
        {NODES.map((label, i) => {
          const x = i * (W + GAP);
          return (
            <g key={label}>
              <rect
                x={x}
                y={24}
                width={W}
                height={72}
                rx={12}
                className={`flow-box${i === 2 ? " flow-box-accent" : ""}`}
              />
              <text
                x={x + W / 2}
                y={56}
                textAnchor="middle"
                className="flow-title-sm"
              >
                {label}
              </text>
              <text
                x={x + W / 2}
                y={78}
                textAnchor="middle"
                className="flow-sub"
              >
                {i === 0
                  ? "~1ms hook client"
                  : i === 1
                    ? "tree-sitter + redact"
                    : i === 2
                      ? "L0 → L1 → L3 → L4"
                      : "approve · block · ask"}
              </text>
              {i < NODES.length - 1 ? (
                <line
                  x1={x + W}
                  y1={60}
                  x2={x + W + GAP - 4}
                  y2={60}
                  className="flow-link"
                  markerEnd="url(#pipe-arrow)"
                />
              ) : null}
            </g>
          );
        })}
      </svg>
    );
  },
);
