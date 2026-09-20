//! Criterion bench for `algo-daemon` Pipeline::decide L0/L1 path
//! `cargo bench -p algo-daemon --bench pipeline_L0L1 -- --save-baseline p1-exit`
//! Group name: `pipeline_L0L1` per spec.
//! Benches:
//!   - L0 policy deny (`rm -rf /`) — hard-deny hot path, no network, no regex beyond OnceLock
//!   - L1 cache hit (`ls -la` after warm) — blake3 + DashMap hit, no Jev
//! Budget: L0/L1 p50<3ms p99<10ms, 100 rps burst; L3 p50<250 p99<800 report-only (mock Jev 700ms timeout).
//! Absolute thresholds + >10% regression fail in scripts/latency-budget.* (scaffold parses criterion JSON).
//! Reference: hyperfine hook-client cold start ~1ms (see scripts/latency-budget.sh).
//! Baselines via `criterion::html_reports` under target/criterion/pipeline_L0L1

use algo_daemon::cache::Cache;
use algo_daemon::jev_pool::JevPool;
use algo_daemon::pipeline::Pipeline;
use algo_provider::MockProvider;
use algo_types::{AgentIdentity, PrivacyMode, ToolBefore, ToolKind};
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

fn tool_before(payload: &str) -> ToolBefore {
    ToolBefore {
        event_id: "bench-evt".into(),
        timestamp: None,
        agent: Some(AgentIdentity {
            agent_type: Some("claude-code".into()),
            agent_version: Some("1.0".into()),
            session_id: "bench-sess".into(),
            working_dir: "/tmp".into(),
        }),
        tool_kind: ToolKind::Shell as i32,
        redacted_payload: payload.to_string(),
        privacy_mode: PrivacyMode::Redacted as i32,
        shell_argv: vec![],
        file_path: None,
    }
}

fn make_pipeline(rt: &Runtime) -> (Pipeline, mpsc::Receiver<algo_daemon::pipeline::DbRecord>) {
    let engine = Arc::new(algo_policy::Engine::new());
    let cache = Arc::new(Cache::new());
    let pool = Arc::new(JevPool::new(Arc::new(MockProvider::new())));
    let (tx, rx) = mpsc::channel(1000);
    let pipeline = Pipeline::new(engine, cache.clone(), pool, tx);
    // Warm L1 cache with a known allow so L1 path can be measured without Jev
    rt.block_on(async {
        // Directly insert a cached decision for ls -la variant (normalize collapses /tmp/*)
        let cached = algo_types::Decision {
            action: algo_types::Action::Allow as i32,
            reason: "cached allow (bench warm)".into(),
            confidence_0_1: 0.9,
            source_level: algo_types::SourceLevel::Jev as i32, // will be overwritten to Cache on get
            latency_ms: 1,
            policy_version: env!("CARGO_PKG_VERSION").to_string(),
            trace_id: "bench-warm".into(),
        };
        // Warm with ls -la and ls -la /tmp/foo to cover two normalized forms
        cache.insert("ls -la", cached.clone());
        cache.insert("ls -la /tmp/foo", cached);
    });
    // Note: caller holds `rx`; bench loop drains in background to avoid channel full timeout
    (pipeline, rx)
}

fn bench_pipeline_l0l1(c: &mut Criterion) {
    let rt = Runtime::new().expect("tokio runtime for bench");
    let (pipeline, mut rx) = make_pipeline(&rt);
    let pipeline = Arc::new(pipeline);

    // Spawn a drain task so writer channel never blocks >1s (otherwise pipeline maps to ask)
    // This mimics the real single-writer task but discards records.
    rt.spawn(async move {
        while let Some(_rec) = rx.recv().await {
            // discard
        }
    });

    let mut group = c.benchmark_group("pipeline_L0L1");
    group.throughput(Throughput::Elements(1));
    // Warm the runtime + one decide to avoid first-iteration cold-start bias
    rt.block_on(async {
        let _ = pipeline.decide(tool_before("ls -la")).await;
        let _ = pipeline.decide(tool_before("rm -rf /")).await;
    });

    // L0: policy deny — rm -rf / (hard deny, should not hit cache or Jev)
    group.bench_function("L0_policy_deny_rm_rf", |b| {
        b.iter(|| {
            let p = pipeline.clone();
            rt.block_on(async {
                let d = p.decide(black_box(tool_before("rm -rf /"))).await;
                black_box(d);
            })
        });
    });

    // L1: cache hit — ls -la (pre-warmed, blake3 + DashMap)
    group.bench_function("L1_cache_hit_ls", |b| {
        b.iter(|| {
            let p = pipeline.clone();
            rt.block_on(async {
                let d = p.decide(black_box(tool_before("ls -la"))).await;
                black_box(d);
            })
        });
    });

    // Mixed L0/L1 burst — 100 sequential decides (measures 100 rps budget)
    group.bench_function("L0L1_throughput_100_mixed", |b| {
        b.iter(|| {
            rt.block_on(async {
                for _ in 0..50 {
                    black_box(pipeline.decide(tool_before("rm -rf /")).await);
                    black_box(pipeline.decide(tool_before("ls -la")).await);
                }
            });
        });
    });

    // L1 only burst — 100 cache hits
    group.bench_function("L1_throughput_100_cache_hit", |b| {
        b.iter(|| {
            rt.block_on(async {
                for _ in 0..100 {
                    black_box(pipeline.decide(tool_before("ls -la")).await);
                }
            });
        });
    });

    group.finish();

    // Hints + threshold scaffold: measure p50 via wall-clock sampling
    let samples = 500usize;
    let start = Instant::now();
    rt.block_on(async {
        for _ in 0..samples {
            black_box(pipeline.decide(tool_before("rm -rf /")).await);
        }
    });
    let avg_ns = start.elapsed().as_nanos() as f64 / samples as f64;
    eprintln!(
        "[pipeline_L0L1] L0 sampled avg ~{:.0} ns ({:.3} ms) budget p50<3ms p99<10ms",
        avg_ns,
        avg_ns / 1_000_000.0
    );
    if avg_ns > 3_000_000.0 {
        eprintln!("[pipeline_L0L1] WARNING: L0 exceeds 3ms p50 budget (spec phase-1-09)");
    }

    let start2 = Instant::now();
    rt.block_on(async {
        for _ in 0..samples {
            black_box(pipeline.decide(tool_before("ls -la")).await);
        }
    });
    let avg2_ns = start2.elapsed().as_nanos() as f64 / samples as f64;
    eprintln!(
        "[pipeline_L0L1] L1 sampled avg ~{:.0} ns ({:.3} ms) budget p50<3ms p99<10ms",
        avg2_ns,
        avg2_ns / 1_000_000.0
    );
    if avg2_ns > 3_000_000.0 {
        eprintln!("[pipeline_L0L1] WARNING: L1 exceeds 3ms p50 budget");
    }

    // L3 mock note (report-only): would be `__sleep_800__` path that hits Jev timeout 700ms
    // We don't bench it as part of L0/L1 budget; latency-budget.sh reports it separately.
    eprintln!(
        "[pipeline_L0L1] L3 mock p50<250ms p99<800ms is report-only (see scripts/latency-budget)"
    );
    eprintln!("[pipeline_L0L1] baseline: cargo bench -p algo-daemon --bench pipeline_L0L1 -- --save-baseline p1-exit");
    // Ensure runtime shuts down gracefully — drop pipeline before criterion exit
    drop(pipeline);
    // Give drain task a moment
    rt.block_on(async { tokio::time::sleep(Duration::from_millis(10)).await });
}

criterion_group!(benches, bench_pipeline_l0l1);
criterion_main!(benches);
