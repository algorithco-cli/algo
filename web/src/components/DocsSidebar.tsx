import {
  Download04Icon,
  Layers01Icon,
  Notification03Icon,
  PaintBoardIcon,
  Settings02Icon,
  TextFontIcon,
} from "@hugeicons/core-free-icons";
import * as React from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { DOCS_SIDEBAR } from "../lib/routes";
import BranchedMenu from "./BranchedMenu";

export function DocsSidebar(): JSX.Element {
  const location = useLocation();
  const navigate = useNavigate();
  const activeValue = React.useMemo(() => {
    if (location.pathname === "/docs/install") return "install";
    if (location.pathname === "/docs/cli") return "cli";
    if (location.pathname === "/privacy") return "privacy";
    return "docs";
  }, [location.pathname]);
  const items = React.useMemo(
    () =>
      DOCS_SIDEBAR.map((g) => ({
        label: g.label,
        children: g.children.map((c) => {
          const iconMap: Record<string, unknown> = {
            docs: TextFontIcon,
            install: Download04Icon,
            cli: Settings02Icon,
            privacy: PaintBoardIcon,
          };
          return {
            value: c.value,
            label: c.label,
            icon: (iconMap[c.value] ?? Layers01Icon) as unknown,
            href: c.to,
          };
        }),
      })),
    [],
  );
  const open = React.useMemo(() => [0, 1, 2], []);
  const onSelect = React.useCallback(
    (value: string) => {
      const found = DOCS_SIDEBAR.flatMap((g) => g.children).find(
        (c) => c.value === value,
      );
      if (found) void navigate(found.to);
    },
    [navigate],
  );
  const onToggle = React.useCallback(() => undefined, []);
  return (
    <BranchedMenu
      ariaLabel="Docs navigation"
      className="branched-menu--docs"
      width={240}
      rowHeight={36}
      indent={40}
      radius={10}
      color="var(--ag-text)"
      accentColor="var(--ag-brand)"
      lineColor="var(--ag-border)"
      fontSize={14}
      items={items}
      active={activeValue}
      open={open}
      onSelect={onSelect}
      onToggle={onToggle}
    />
  );
}
