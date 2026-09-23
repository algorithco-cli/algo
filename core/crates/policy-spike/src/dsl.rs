//! DSL path (b): minimal pest `deny if ...` language. Parse-once at load,
//! eval per decision. Parse/build error → Ask (fail-safe, never Allow).

use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

use crate::facts::Facts;

#[derive(Parser)]
#[grammar = "dsl.pest"]
struct DslParser;

#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    Deny,
    Abstain,
    Ask,
}

pub struct DslEngine {
    rules: Vec<(String, Compiled)>,
}

#[derive(Debug, Clone)]
enum Compiled {
    Ast(Expr),
}

#[derive(Debug, Clone)]
enum Expr {
    Or(Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
    InString {
        needle: String,
        list: ListRef,
    },
    Contains {
        needle: String,
    },
    EqBool {
        var: BoolRef,
        expect: bool,
        negate: bool,
    },
}

#[derive(Debug, Clone, Copy)]
enum ListRef {
    Bin,
    Flag,
    Tok,
}

#[derive(Debug, Clone, Copy)]
enum BoolRef {
    PipeToShell,
    Net,
}

fn unquote(s: &str) -> String {
    s.strip_prefix('"')
        .and_then(|t| t.strip_suffix('"'))
        .unwrap_or(s)
        .to_string()
}

fn parse_expr(pair: Pair<Rule>) -> Result<Expr, String> {
    // expr = { or } — single child.
    let inner = pair.into_inner().next().ok_or("empty expr")?;
    parse_or(inner)
}

fn parse_or(pair: Pair<Rule>) -> Result<Expr, String> {
    // or = { and ~ ("||" ~ and)* } — anonymous "||" is silent; children are `and`.
    let mut inner = pair.into_inner();
    let first = parse_and(inner.next().ok_or("empty or")?)?;
    let mut acc = first;
    for next in inner {
        debug_assert_eq!(next.as_rule(), Rule::and);
        let rhs = parse_and(next)?;
        acc = Expr::Or(Box::new(acc), Box::new(rhs));
    }
    Ok(acc)
}

fn parse_and(pair: Pair<Rule>) -> Result<Expr, String> {
    let mut inner = pair.into_inner();
    let first = parse_unary(inner.next().ok_or("empty and")?)?;
    let mut acc = first;
    for next in inner {
        debug_assert_eq!(next.as_rule(), Rule::unary);
        let rhs = parse_unary(next)?;
        acc = Expr::And(Box::new(acc), Box::new(rhs));
    }
    Ok(acc)
}

fn parse_unary(pair: Pair<Rule>) -> Result<Expr, String> {
    let mut inner = pair.into_inner();
    let first = inner.next().ok_or("empty unary")?;
    match first.as_rule() {
        Rule::unary => Ok(Expr::Not(Box::new(parse_unary(first)?))),
        Rule::primary => parse_primary(first),
        r => Err(format!("unexpected unary child: {r:?}")),
    }
}

fn parse_primary(pair: Pair<Rule>) -> Result<Expr, String> {
    let inner = pair.into_inner().next().ok_or("empty primary")?;
    match inner.as_rule() {
        Rule::expr => parse_expr(inner),
        Rule::pred => parse_pred(inner),
        r => Err(format!("unexpected primary child: {r:?}")),
    }
}

fn parse_pred(pair: Pair<Rule>) -> Result<Expr, String> {
    let inner = pair.into_inner().next().ok_or("empty pred")?;
    match inner.as_rule() {
        Rule::in_pred => {
            let mut it = inner.into_inner();
            let needle = unquote(it.next().ok_or("in: no string")?.as_str());
            let list = match it.next().ok_or("in: no list")?.as_str() {
                "bin" => ListRef::Bin,
                "flag" => ListRef::Flag,
                "tok" => ListRef::Tok,
                other => return Err(format!("unknown list: {other}")),
            };
            Ok(Expr::InString { needle, list })
        }
        Rule::contains_pred => {
            let mut it = inner.into_inner();
            match it.next().ok_or("contains: no ident")?.as_str() {
                "raw" => {}
                other => return Err(format!("contains only on raw, got: {other}")),
            }
            let needle = unquote(it.next().ok_or("contains: no string")?.as_str());
            Ok(Expr::Contains { needle })
        }
        Rule::eq_pred => {
            let mut it = inner.into_inner();
            let var = match it.next().ok_or("eq: no ident")?.as_str() {
                "pipe_to_shell" => BoolRef::PipeToShell,
                "net" => BoolRef::Net,
                other => return Err(format!("eq only on bool vars, got: {other}")),
            };
            let negate = it.next().ok_or("eq: no op")?.as_str() == "!=";
            let expect = it.next().ok_or("eq: no bool")?.as_str() == "true";
            Ok(Expr::EqBool {
                var,
                expect,
                negate,
            })
        }
        r => Err(format!("unexpected pred child: {r:?}")),
    }
}

fn eval(e: &Expr, f: &Facts) -> bool {
    match e {
        Expr::Or(a, b) => eval(a, f) || eval(b, f),
        Expr::And(a, b) => eval(a, f) && eval(b, f),
        Expr::Not(x) => !eval(x, f),
        Expr::InString { needle, list } => {
            let hay: &[String] = match list {
                ListRef::Bin => &f.bins,
                ListRef::Flag => &f.flags,
                ListRef::Tok => &f.toks,
            };
            hay.iter().any(|t| t == needle)
        }
        Expr::Contains { needle } => f.raw.contains(needle.as_str()),
        Expr::EqBool {
            var,
            expect,
            negate,
        } => {
            let v = match var {
                BoolRef::PipeToShell => f.has_pipe_to_shell,
                BoolRef::Net => f.net,
            };
            (v == *expect) != *negate
        }
    }
}

impl DslEngine {
    /// Parse-once. Returns engine + (id, error) for rules that failed —
    /// caller treats missing rules as Ask, never Allow.
    pub fn compile_all(srcs: &[(&str, &str)]) -> (Self, Vec<(String, String)>) {
        let mut rules = Vec::with_capacity(srcs.len());
        let mut errors = Vec::new();
        for (id, src) in srcs {
            match DslParser::parse(Rule::rule, src) {
                Ok(mut pairs) => {
                    let rule_pair = pairs.next().unwrap();
                    // rule = { SOI ~ "deny" ~ "if" ~ expr ~ EOI } → find expr child.
                    let expr_pair = rule_pair.into_inner().find(|p| p.as_rule() == Rule::expr);
                    match expr_pair {
                        Some(ep) => match parse_expr(ep) {
                            Ok(ast) => rules.push((id.to_string(), Compiled::Ast(ast))),
                            Err(e) => errors.push((id.to_string(), e)),
                        },
                        None => errors.push((id.to_string(), "no expr".into())),
                    }
                }
                Err(e) => errors.push((id.to_string(), format!("{e}"))),
            }
        }
        (Self { rules }, errors)
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Deny if any rule evals true. Empty set → Ask.
    pub fn evaluate(&self, facts: &Facts) -> Verdict {
        if self.rules.is_empty() {
            return Verdict::Ask;
        }
        for (_, Compiled::Ast(ast)) in &self.rules {
            if eval(ast, facts) {
                return Verdict::Deny;
            }
        }
        Verdict::Abstain
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus;

    fn engine_13() -> DslEngine {
        let real = corpus::real_rules();
        let srcs: Vec<(&str, &str)> = real.iter().map(|r| (r.id, r.dsl.as_str())).collect();
        let (e, errors) = DslEngine::compile_all(&srcs);
        assert!(errors.is_empty(), "real DSL rules must parse: {errors:?}");
        assert_eq!(e.rule_count(), 13);
        e
    }

    #[test]
    fn dsl_13_deny_and_safe_abstain() {
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
    fn proves_ask_on_dsl_parse_error() {
        let (e, errors) = DslEngine::compile_all(&[("BAD", "deny iff blah blah")]);
        assert_eq!(errors.len(), 1);
        assert_eq!(e.evaluate(&crate::facts::facts_of("ls")), Verdict::Ask);
    }

    #[test]
    fn proves_ask_on_dsl_type_error() {
        // Bool equality on a list-typed var is rejected at AST build → Ask.
        let (e, errors) = DslEngine::compile_all(&[("TYP", "deny if bin == true")]);
        assert_eq!(errors.len(), 1);
        assert_eq!(e.evaluate(&crate::facts::facts_of("ls")), Verdict::Ask);
    }

    #[test]
    fn dsl_malformed_input_never_panics() {
        let garbage = [
            "deny if ",
            "deny if ((((((",
            "allow if true",
            "\u{0}\u{0}",
            "deny if \"unterminated",
            "deny if bin in \"rm\"",
            "deny if !!!!!",
            "deny if raw contains",
        ];
        for g in garbage {
            let r = std::panic::catch_unwind(|| DslParser::parse(Rule::rule, g));
            assert!(r.is_ok(), "DSL parse must not panic on: {g:?}");
        }
    }
}
