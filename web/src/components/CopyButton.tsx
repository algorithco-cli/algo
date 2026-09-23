import Check from "lucide-react/icons/check.mjs";
import Copy from "lucide-react/icons/copy.mjs";
import * as React from "react";

export function CopyButton({ text }: { text: string }): JSX.Element {
  const [copied, setCopied] = React.useState(false);
  const timer = React.useRef(0);
  React.useEffect(() => () => window.clearTimeout(timer.current), []);
  const onCopy = React.useCallback(() => {
    void navigator.clipboard?.writeText(text).then(
      () => {
        setCopied(true);
        window.clearTimeout(timer.current);
        timer.current = window.setTimeout(() => setCopied(false), 1600);
      },
      () => undefined,
    );
  }, [text]);
  return (
    <button
      type="button"
      className="btn btn-ghost copy-btn"
      onClick={onCopy}
      aria-label="Copy install command"
    >
      {copied ? (
        <Check size={14} aria-hidden />
      ) : (
        <Copy size={14} aria-hidden />
      )}
      {copied ? "Copied" : "Copy"}
    </button>
  );
}
