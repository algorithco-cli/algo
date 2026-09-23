import * as React from "react";
import { CopyButton } from "./CopyButton";

const INSTALL_TABS = ["Claude Code", "Codex CLI", "OpenCode"] as const;

export function InstallTabs(): JSX.Element {
  const [tab, setTab] = React.useState(0);
  const tabsRef = React.useRef<Array<HTMLButtonElement | null>>([]);
  const onKeyDown = React.useCallback(
    (e: React.KeyboardEvent): void => {
      let next: number | null = null;
      if (e.key === "ArrowRight" || e.key === "ArrowDown")
        next = (tab + 1) % INSTALL_TABS.length;
      else if (e.key === "ArrowLeft" || e.key === "ArrowUp")
        next = (tab - 1 + INSTALL_TABS.length) % INSTALL_TABS.length;
      else if (e.key === "Home") next = 0;
      else if (e.key === "End") next = INSTALL_TABS.length - 1;
      if (next === null) return;
      e.preventDefault();
      setTab(next);
      tabsRef.current[next]?.focus();
    },
    [tab],
  );
  const setTabCb = React.useCallback((i: number) => setTab(i), []);
  return (
    <div className="card install-card">
      <div
        className="install-tabs"
        role="tablist"
        aria-label="Install per agent"
        onKeyDown={onKeyDown}
      >
        {INSTALL_TABS.map((name, i) => (
          <button
            type="button"
            key={name}
            ref={(el) => {
              tabsRef.current[i] = el;
            }}
            role="tab"
            aria-selected={i === tab}
            aria-controls={`install-panel-${i}`}
            id={`install-tab-${i}`}
            tabIndex={i === tab ? 0 : -1}
            className={`install-tab${i === tab ? " active" : ""}`}
            onClick={() => setTabCb(i)}
          >
            {name}
            {i === 0 ? (
              <span className="badge badge-allow">now</span>
            ) : (
              <span className="badge badge-ask">planned</span>
            )}
          </button>
        ))}
      </div>
      <div
        role="tabpanel"
        id={`install-panel-${tab}`}
        aria-labelledby={`install-tab-${tab}`}
        className="install-panel"
      >
        {tab === 0 ? (
          <>
            <pre>
              <code>{`curl -fsSL http://127.0.0.1:3007/install.sh | sh
algo init   # ~30s: detect agents → diff → consent → hooks`}</code>
            </pre>
            <CopyButton text="curl -fsSL http://127.0.0.1:3007/install.sh | sh" />
          </>
        ) : (
          <p className="muted">
            {INSTALL_TABS[tab]} adapter is planned (Phase 4). The daemon speaks
            a versioned proto contract so adapters stay thin. Start with Claude
            Code today.
          </p>
        )}
      </div>
    </div>
  );
}
