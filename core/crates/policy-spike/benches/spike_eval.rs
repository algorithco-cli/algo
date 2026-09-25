//! Criterion benches for the P1-01 spike: compile-once time (500 rules)
//! and eval p50/p99 per decision (500 rules over all sample fact-sets).
//! Budgets under test: L0/L1 p50<3ms / p99<10ms per decision.

use criterion::{criterion_group, criterion_main, Criterion};
use std::time::Duration;

use algo_policy_spike::cel_engine::CelEngine;
use algo_policy_spike::corpus;
use algo_policy_spike::dsl::DslEngine;
use algo_policy_spike::facts::facts_of;

fn corpus_500_cel() -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = corpus::real_rules()
        .iter()
        .map(|r| (r.id.to_string(), r.cel.clone()))
        .collect();
    for (i, s) in corpus::synthetic_cel(487).into_iter().enumerate() {
        v.push((format!("SYN{i:03}"), s));
    }
    v
}

fn corpus_500_dsl() -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = corpus::real_rules()
        .iter()
        .map(|r| (r.id.to_string(), r.dsl.clone()))
        .collect();
    for (i, s) in corpus::synthetic_dsl(487).into_iter().enumerate() {
        v.push((format!("SYN{i:03}"), s));
    }
    v
}

fn sample_facts() -> Vec<algo_policy_spike::facts::Facts> {
    corpus::sample_cmds()
        .into_iter()
        .map(|(cmd, _)| facts_of(cmd))
        .collect()
}

fn bench_compile(c: &mut Criterion) {
    let cel = corpus_500_cel();
    let cel_refs: Vec<(&str, &str)> = cel.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    c.bench_function("cel_compile_once_500", |b| {
        b.iter(|| {
            let (e, errors) = CelEngine::compile_all(std::hint::black_box(&cel_refs));
            assert!(errors.is_empty());
            assert_eq!(e.rule_count(), 500);
            e
        });
    });

    let dsl = corpus_500_dsl();
    let dsl_refs: Vec<(&str, &str)> = dsl.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    c.bench_function("dsl_parse_once_500", |b| {
        b.iter(|| {
            let (e, errors) = DslEngine::compile_all(std::hint::black_box(&dsl_refs));
            assert!(errors.is_empty());
            assert_eq!(e.rule_count(), 500);
            e
        });
    });
}

fn bench_eval(c: &mut Criterion) {
    let cel = corpus_500_cel();
    let cel_refs: Vec<(&str, &str)> = cel.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    let (cel_engine, errors) = CelEngine::compile_all(&cel_refs);
    assert!(errors.is_empty());

    let dsl = corpus_500_dsl();
    let dsl_refs: Vec<(&str, &str)> = dsl.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    let (dsl_engine, errors) = DslEngine::compile_all(&dsl_refs);
    assert!(errors.is_empty());

    let facts = sample_facts();

    c.bench_function("cel_eval_500rules_allfacts", |b| {
        b.iter(|| {
            for f in &facts {
                std::hint::black_box(cel_engine.evaluate(std::hint::black_box(f)));
            }
        });
    });

    c.bench_function("dsl_eval_500rules_allfacts", |b| {
        b.iter(|| {
            for f in &facts {
                std::hint::black_box(dsl_engine.evaluate(std::hint::black_box(f)));
            }
        });
    });

    // Per-decision latency: single eval over 500 rules (the L0/L1 budget test).
    // NOTE facts[0] ("rm -rf /") hits the FIRST rule → short-circuit best
    // case. Worst case is a safe input (scans all 500) — measured below.
    let one = &facts[0];
    c.bench_function("cel_eval_single_decision_500rules", |b| {
        b.iter(|| std::hint::black_box(cel_engine.evaluate(std::hint::black_box(one))));
    });
    c.bench_function("dsl_eval_single_decision_500rules", |b| {
        b.iter(|| std::hint::black_box(dsl_engine.evaluate(std::hint::black_box(one))));
    });
    let safe = facts_of("ls -la --color=auto /tmp");
    c.bench_function("cel_eval_single_safe_500rules", |b| {
        b.iter(|| std::hint::black_box(cel_engine.evaluate(std::hint::black_box(&safe))));
    });
    c.bench_function("dsl_eval_single_safe_500rules", |b| {
        b.iter(|| std::hint::black_box(dsl_engine.evaluate(std::hint::black_box(&safe))));
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(5));
    targets = bench_compile, bench_eval
);
criterion_main!(benches);
