//! CEL path (a): `cel` crate (cel-rust). Compile-once at load, eval per
//! decision. Any compile/exec error maps to Ask (fail-safe, never Allow).

use cel::{Context, Program};

use crate::facts::Facts;

#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    Deny,
    Abstain,
    Ask,
}

pub struct CelEngine {
    programs: Vec<(String, Program)>,
}

impl CelEngine {
    /// Compile-once. Returns the engine plus (id, error) for rules that
    /// failed to compile — caller treats missing rules as Ask, never Allow.
    pub fn compile_all(srcs: &[(&str, &str)]) -> (Self, Vec<(String, String)>) {
        let mut programs = Vec::with_capacity(srcs.len());
        let mut errors = Vec::new();
        for (id, src) in srcs {
            match Program::compile(src) {
                Ok(p) => programs.push((id.to_string(), p)),
                Err(e) => errors.push((id.to_string(), format!("{e:?}"))),
            }
        }
        (Self { programs }, errors)
    }

    pub fn rule_count(&self) -> usize {
        self.programs.len()
    }

    fn context_for(facts: &Facts) -> Context<'static> {
        // Per-eval context (measured cost, see README + bench).
        let mut ctx = Context::default();
        let _ = ctx.add_variable("bins", facts.bins.clone());
        let _ = ctx.add_variable("flags", facts.flags.clone());
        let _ = ctx.add_variable("toks", facts.toks.clone());
        let _ = ctx.add_variable("has_pipe_to_shell", facts.has_pipe_to_shell);
        let _ = ctx.add_variable("net", facts.net);
        let _ = ctx.add_variable("raw", facts.raw.clone());
        ctx
    }

    /// Deny if any program evals true. Exec error on any rule → Ask.
    /// Empty program set → Ask (no rule compiled is not an allow).
    pub fn evaluate(&self, facts: &Facts) -> Verdict {
        if self.programs.is_empty() {
            return Verdict::Ask;
        }
        let ctx = Self::context_for(facts);
        for (_, p) in &self.programs {
            match p.execute(&ctx) {
                Ok(v) => {
                    if v == true.into() {
                        return Verdict::Deny;
                    }
                }
                Err(_) => return Verdict::Ask,
            }
        }
        Verdict::Abstain
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus;

    fn engine_13() -> CelEngine {
        let real = corpus::real_rules();
        let srcs: Vec<(&str, &str)> = real.iter().map(|r| (r.id, r.cel.as_str())).collect();
        let (e, errors) = CelEngine::compile_all(&srcs);
        assert!(errors.is_empty(), "real CEL rules must compile: {errors:?}");
        assert_eq!(e.rule_count(), 13);
        e
    }

    #[test]
    fn cel_13_deny_and_safe_abstain() {
        let e = engine_13();
        for (cmd, dangerous) in corpus::sample_cmds() {
            let f = crate::facts::facts_of(cmd);
            match e.evaluate(&f) {
                Verdict::Deny => assert!(dangerous, "false deny on safe: {cmd}"),
                Verdict::Abstain => assert!(!dangerous, "missed dangerous: {cmd}"),
                Verdict::Ask => panic!("unexpected Ask on sample: {cmd}"),
            }
        }
    }

    #[test]
    fn proves_ask_on_cel_compile_error() {
        let (e, errors) = CelEngine::compile_all(&[("BAD", "this is not (cel")]);
        assert_eq!(errors.len(), 1);
        // No program compiled → Ask, never Allow.
        assert_eq!(e.evaluate(&crate::facts::facts_of("ls")), Verdict::Ask);
    }

    #[test]
    fn proves_ask_on_cel_exec_error() {
        // Unknown variable → execution error → Ask.
        let (e, errors) = CelEngine::compile_all(&[("UNK", "nope_var == true")]);
        assert!(errors.is_empty());
        assert_eq!(e.evaluate(&crate::facts::facts_of("ls")), Verdict::Ask);
    }

    #[test]
    fn cel_malformed_input_never_panics() {
        // CVE-2025-62162 (parser panic on malformed input, fixed ≥0.11.4):
        // every garbage input must return Err, never panic.
        let garbage = [
            "((((((((((",
            "\u{0}\u{0}\u{0}",
            "has(macro.",
            "[1, 2, }",
            "a.?b.?c",
            "'''\\",
            "123abc def",
            "? : ?",
        ];
        for g in garbage {
            let _ = std::panic::catch_unwind(|| Program::compile(g))
                .expect("CEL compile must not panic (CVE-2025-62162)");
        }
    }

    #[test]
    fn cel_references_auditable() {
        // Auditability datum: compiled programs expose their variable /
        // function references (bundle review tooling can diff these).
        let p = Program::compile(r#""rm" in bins && raw.contains("x")"#).unwrap();
        let refs = p.references();
        assert!(refs.has_variable("bins"));
        assert!(refs.has_variable("raw"));
    }

    #[test]
    fn cel_matches_support_probe() {
        // Datum for the ADR expressiveness row: does this `cel` version
        // support the standard `matches()` macro? Record, don't gate.
        let r = Program::compile(r#"raw.matches("mkfs")"#);
        eprintln!("PROBE cel matches() supported: {}", r.is_ok());
    }
}
