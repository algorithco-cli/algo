import { CheckmarkCircle02Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useState } from "react";
import { Link } from "react-router-dom";
import ElectricBorder from "../components/ElectricBorder";
import { Section } from "../components/Section";
import { Seo } from "../components/Seo";
import { useRevealOnMount } from "../hooks/useRevealOnMount";

interface PlanCta {
  to: string;
  label: string;
  primary: boolean;
}

interface Plan {
  id: string;
  name: string;
  buyer: string;
  monthly: number;
  annual: number;
  saveYr: string | null;
  perAnnual: string;
  perMonthly: string;
  features: string[];
  cta: PlanCta;
  micro: string;
}

const PLANS: readonly Plan[] = [
  {
    id: "pro",
    name: "Pro",
    buyer: "For pros who live in the terminal.",
    monthly: 12,
    annual: 10,
    saveYr: "$24",
    perAnnual: "per seat / mo · billed annually",
    perMonthly: "per seat / mo · billed monthly",
    features: [
      "Everything in Free, plus:",
      "Dashboard + SSE live tail",
      "Signed bundles + dry-run enforce",
      "Per-user stats",
    ],
    cta: { to: "/login", label: "Start with Pro", primary: true },
    micro: "Per seat · cancel anytime",
  },
  {
    id: "max",
    name: "Max",
    buyer: "For growing teams sharing one policy.",
    monthly: 29,
    annual: 24,
    saveYr: "$60",
    perAnnual: "per seat / mo · billed annually",
    perMonthly: "per seat / mo · billed monthly",
    features: [
      "Everything in Pro, plus:",
      "OAuth + org workspaces",
      "Audit export for SIEM",
      "Early access to new rules",
    ],
    cta: { to: "/login", label: "Start with Max", primary: false },
    micro: "Per seat · cancel anytime",
  },
  {
    id: "team",
    name: "Team",
    buyer: "For orgs with compliance and review gates.",
    monthly: 49,
    annual: 41,
    saveYr: "$96",
    perAnnual: "per seat / mo · billed annually",
    perMonthly: "per seat / mo · billed monthly",
    features: [
      "Everything in Max, plus:",
      "SSO / SCIM provisioning",
      "DPA, ZDR, SBOM + provenance",
      "Dedicated support + security review",
    ],
    cta: { to: "/login", label: "Start with Team", primary: false },
    micro: "Annual contract available",
  },
];

function planAmount(p: Plan, annual: boolean): string {
  if (p.monthly === 0) return "$0";
  return `$${annual ? p.annual : p.monthly}`;
}

interface CompareRow {
  label: string;
  values: readonly (string | boolean)[];
}

interface CompareGroup {
  title: string;
  rows: readonly CompareRow[];
}

const COMPARE: readonly CompareGroup[] = [
  {
    title: "Protection",
    rows: [
      {
        label: "Hard-deny rules (13)",
        values: ["13 rules", "13 rules", "13 rules"],
      },
      { label: "Shadow mode", values: [true, true, true] },
      { label: "Ask-on-error fail-safe", values: [true, true, true] },
      { label: "Signed policy bundles", values: [false, true, true] },
      { label: "Dry-run enforce", values: [false, true, true] },
    ],
  },
  {
    title: "Visibility",
    rows: [
      { label: "algo why audit log", values: [true, true, true] },
      {
        label: "Dashboard + SSE live tail",
        values: [false, true, true],
      },
      { label: "Per-user stats", values: [false, true, true] },
      { label: "Audit export (SIEM)", values: [false, false, true] },
    ],
  },
  {
    title: "Team & compliance",
    rows: [
      { label: "OAuth + org", values: [false, false, true] },
      { label: "SSO / SCIM", values: [false, false, true] },
      { label: "DPA + ZDR", values: [false, false, true] },
      { label: "SBOM + provenance", values: [false, false, true] },
    ],
  },
];

const FAQS: readonly { q: string; a: string }[] = [
  {
    q: "Is there a free option?",
    a: "Yes — the local build runs 100% on your machine: no account, no checkout, no telemetry. Install it free and upgrade only if you need a team plan.",
  },
  {
    q: "Can I cancel a paid plan anytime?",
    a: "Yes. Monthly billing, cancel in two clicks, and you keep Free forever. No retention calls, no dark patterns.",
  },
  {
    q: "How does billing work?",
    a: "Per seat, per month, via an external merchant of record (ADR-0006). Card data never touches our systems — the MoR handles checkout, invoices, and tax.",
  },
  {
    q: "What happens to my code on paid plans?",
    a: "Same redaction-first promise as Free: local-only by default, with BYOK redacted mode strictly opt-in. Nothing trains on your input.",
  },
  {
    q: "What counts as a seat?",
    a: "One seat per developer running the guard. CI machines and shared runners don't need seats.",
  },
];

function Check(): JSX.Element {
  return (
    <HugeiconsIcon icon={CheckmarkCircle02Icon} size={16} strokeWidth={2} />
  );
}

export default function Pricing(): JSX.Element {
  useRevealOnMount();
  const [annual, setAnnual] = useState(true);
  return (
    <>
      <Seo path="/pricing" />
      <Section
        id="pricing"
        kicker="Pricing"
        title="Start free."
        accent="Scale when you trust it."
        lede="Pro, Max, and Team per seat for pros, teams, and orgs — plus a free local build. Cancel anytime."
      >
        <div className="billing-toggle-wrap">
          <fieldset className="billing-toggle">
            <legend className="vdemo-sr">Billing period</legend>
            <button
              type="button"
              data-active={!annual ? "" : undefined}
              aria-pressed={!annual}
              onClick={() => setAnnual(false)}
            >
              Monthly
            </button>
            <button
              type="button"
              data-active={annual ? "" : undefined}
              aria-pressed={annual}
              onClick={() => setAnnual(true)}
            >
              Annual
              <span className="billing-save">−17%</span>
            </button>
          </fieldset>
        </div>
        <div className="pricing-bleed">
          <div className="grid4">
            {PLANS.map((p) => (
              <ElectricBorder
                key={p.id}
                speed={0.7}
                chaos={0.14}
                thickness={2}
                borderRadius={16}
              >
                <div
                  className="card price"
                  id={p.id}
                  style={{
                    border: 0,
                    boxShadow: "none",
                    background: "transparent",
                  }}
                >
                  <h3>{p.name}</h3>
                  <p className="muted plan-buyer">{p.buyer}</p>
                  <p className="plan-price-row">
                    <span className="price-tag">{planAmount(p, annual)}</span>
                    <span className="plan-percol">
                      <span className="muted plan-per">
                        {annual ? p.perAnnual : p.perMonthly}
                      </span>
                      {annual && p.saveYr ? (
                        <span className="plan-total">
                          ${p.annual * 12}/yr ·{" "}
                          <span className="plan-save">save {p.saveYr}</span>
                        </span>
                      ) : null}
                    </span>
                  </p>
                  <ul className="check-list plan-features">
                    {p.features.map((f) => (
                      <li key={f}>
                        <Check />
                        <span>{f}</span>
                      </li>
                    ))}
                  </ul>
                  <Link
                    to={`/billing?plan=${p.id}&cycle=${annual ? "annual" : "monthly"}`}
                    className={`btn plan-cta${p.cta.primary ? " btn-primary" : " btn-ghost"}`}
                  >
                    {p.cta.label}
                  </Link>
                  <p className="muted small plan-micro">{p.micro}</p>
                </div>
              </ElectricBorder>
            ))}
          </div>
        </div>

        <div className="plan-compare">
          <h2 className="plan-compare-title">Compare plans</h2>
          <div className="compare-scroll">
            <table className="compare">
              <thead>
                <tr>
                  <th scope="col">
                    <span className="vdemo-sr">Feature</span>
                  </th>
                  {PLANS.map((p) => (
                    <th key={p.id} scope="col">
                      {p.name}
                      <span className="compare-plan-sub">
                        {planAmount(p, annual)}
                      </span>
                    </th>
                  ))}
                </tr>
              </thead>
              {COMPARE.map((g) => (
                <tbody key={g.title}>
                  <tr className="compare-group">
                    <th scope="rowgroup" colSpan={4}>
                      {g.title}
                    </th>
                  </tr>
                  {g.rows.map((r) => (
                    <tr key={r.label}>
                      <th scope="row">{r.label}</th>
                      {r.values.map((v, i) => (
                        <td key={`${r.label}-${PLANS[i]?.id ?? i}`}>
                          {typeof v === "string" ? (
                            <span className="compare-value">{v}</span>
                          ) : v ? (
                            <span className="compare-yes">
                              <Check />
                              <span className="vdemo-sr">Included</span>
                            </span>
                          ) : (
                            <span
                              className="compare-na"
                              aria-label="Not included"
                            >
                              —
                            </span>
                          )}
                        </td>
                      ))}
                    </tr>
                  ))}
                </tbody>
              ))}
            </table>
          </div>
        </div>

        <div className="plan-faq">
          <h2 className="plan-compare-title">Pricing questions</h2>
          {FAQS.map((f) => (
            <details key={f.q} className="price-faq">
              <summary>{f.q}</summary>
              <p className="muted">{f.a}</p>
            </details>
          ))}
        </div>

        <p className="muted small plan-footnote">
          Paid plans billed per seat via external MoR (ADR-0006). See{" "}
          <Link to="/roadmap">roadmap</Link> and <Link to="/faq">FAQ</Link>.
        </p>
      </Section>
    </>
  );
}
