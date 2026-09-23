import * as React from "react";

export function LoginMock(): JSX.Element {
  const [step, setStep] = React.useState<0 | 1 | 2>(0);
  const [code] = React.useState(() => {
    const chars = "ABCDEFGHJKMNPQRSTUVWXYZ23456789";
    let s = "";
    for (let i = 0; i < 8; i += 1)
      s += chars[Math.floor(Math.random() * chars.length)];
    return `${s.slice(0, 4)}-${s.slice(4)}`;
  });
  const [typed, setTyped] = React.useState("");
  React.useEffect(() => {
    if (step !== 1) return;
    if (typed.trim().toUpperCase() !== code) return;
    const t = window.setTimeout(() => setStep(2), 900);
    return () => window.clearTimeout(t);
  }, [typed, code, step]);
  const onTyped = React.useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => setTyped(e.target.value),
    [],
  );
  const toStep1 = React.useCallback(() => setStep(1), []);
  return (
    <div className="card login-card">
      <div className="login-steps">
        <div className={`login-step${step >= 0 ? " active" : ""}`}>
          <span className="step-n">1</span>
          <div>
            <strong>Run in your terminal</strong>
            <pre className="login-pre">
              <code>$ algo login{"\n"}→ visit this page and enter code</code>
            </pre>
            <p className="device-code">
              Your code: <code>{code}</code>
            </p>
            {step === 0 ? (
              <button
                type="button"
                className="btn btn-primary"
                onClick={toStep1}
              >
                I ran it — continue
              </button>
            ) : null}
          </div>
        </div>
        <div className={`login-step${step >= 1 ? " active" : ""}`}>
          <span className="step-n">2</span>
          <div>
            <strong>Enter the device code</strong>
            <div className="demo-row">
              <input
                className="demo-input"
                value={typed}
                onChange={onTyped}
                placeholder="XXXX-XXXX"
                spellCheck={false}
                autoComplete="off"
                disabled={step !== 1}
              />
            </div>
            {step === 1 &&
            typed.trim() !== "" &&
            typed.trim().toUpperCase() !== code ? (
              <p className="muted login-hint">
                That doesn&apos;t match — check the code above.
              </p>
            ) : null}
          </div>
        </div>
        <div className={`login-step${step >= 2 ? " active" : ""}`}>
          <span className="step-n">3</span>
          <div>
            <strong>Linked</strong>
            {step === 2 ? (
              <p className="login-done">
                <span className="badge badge-allow">allow</span> Device linked —{" "}
                <code>algo status</code> now shows your org policy. Team cloud
                is Phase 3 (planned); today everything still runs local-first.
              </p>
            ) : (
              <p className="muted login-hint">Waiting for the code…</p>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
