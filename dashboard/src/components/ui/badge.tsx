import type * as React from "react";
import { Action } from "../../lib/api";
import { cn } from "../../lib/utils";

type BadgeProps = React.HTMLAttributes<HTMLSpanElement> & {
  action: Action;
};

// Decision colors are semantic and exclusive: allow/ask/deny must use
// --ag-allow/--ag-ask/--ag-deny only (design-tokens.md Variant 1).
// Never reuse for branding.

function actionClass(a: Action): string {
  switch (a) {
    case Action.ACTION_ALLOW:
      return "badge-allow";
    case Action.ACTION_DENY:
      return "badge-deny";
    default:
      return "badge-ask";
  }
}

function actionLabel(a: Action): string {
  switch (a) {
    case Action.ACTION_ALLOW:
      return "allow";
    case Action.ACTION_DENY:
      return "deny";
    default:
      return "ask";
  }
}

export function DecisionBadge({
  action,
  className,
  ...props
}: BadgeProps): JSX.Element {
  return (
    <span
      className={cn(
        "inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-semibold tracking-wide",
        actionClass(action),
        className,
      )}
      {...props}
    >
      {actionLabel(action)}
    </span>
  );
}
