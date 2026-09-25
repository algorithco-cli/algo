//! Criterion bench for `algo-redact` Redactor::redact on 10KB payload with secrets
//! `cargo bench -p algo-redact --bench redact_10k -- --save-baseline p1-exit`
//! Group name: `redact_10k` per spec.
//! Budget: crate spec <500us/10KB, pipeline off-path aho-corasick only in redact.
//! Enforced via scripts/latency-budget.* (+ proptest/fuzz elsewhere).
//! Baselines: html_reports + json under target/criterion/redact_10k/*/new/estimates.json

use algo_redact::Redactor;
use criterion::{criterion_group, criterion_main, Criterion, Throughput};

/// Build a deterministic 10 KiB payload containing secrets interleaved with filler.
/// Contains: AWS key, Github PAT, Slack token, PEM block, vendor SK, JWT-like, credential assignment, high-entropy.
fn payload_10k() -> String {
    // Secret samples (synthetic, not real). High-entropy detector intentionally skips `example`/`test`/`fake`,
    // so we use non-example-looking tokens.
    const SECRETS: &[&str] = &[
        "AKIAIOSFODNN7EXAMPLE", // aws_key (example, but still pattern)
        "AKIAABCDEFGHIJKLMNOP", // 16 chars after AKIA -> triggers aws_key
        "ghp_1234567890abcdef1234567890ABCDEF1234", // github_pat
        "gho_1234567890abcdef1234567890ABCDEF1234",
        "xoxb-1234567890-1234567890-AbCdEfGhIjKlMn", // slack_token
        "-----BEGIN RSA PRIVATE KEY----- MIIE placeholder",
        "sk-live-1234567890abcdef12345678",
        "sk-test-1234567890abcdef12345678",
        "AIza1234567890abcdef1234567890abcdef123", // google_api 35 after AIza
        "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dummySignatureAbcdef123456",
        "password=SuperSecret123! token: abcdef1234567890",
        "aws_secret_access_key = wJalrXUtnFEMI/K7MDENG/bPxRfiCYABCDEFGHIJK",
        // high-entropy dense token (24+, 3 classes, entropy>4.5) without literal prefix
        "aB3dEfGhIjKlMnOpQrStUvWxYz1234567890!@#aB3dEfGhIj",
    ];

    let mut out = String::with_capacity(10 * 1024);
    let mut idx = 0usize;
    while out.len() < 10 * 1024 {
        let filler = format!(" lorem ipsum dolor sit amet block-{} ", idx);
        out.push_str(&filler);
        // Inject a secret every ~700 bytes (~14 blocks)
        if idx % 7 == 0 {
            let secret = SECRETS[idx % SECRETS.len()];
            out.push_str(secret);
            out.push(' ');
        }
        // Add some structured JSON-like payload
        if idx % 3 == 0 {
            out.push_str(r#"{"user":"alice","token":"ghp_1234567890abcdef1234567890ABCDEF1234","path":"/home/alice/.aws/credentials"} "#);
        }
        idx += 1;
    }
    out.truncate(10 * 1024);
    out
}

fn bench_redact_10k(c: &mut Criterion) {
    let redactor = Redactor::new();
    let payload = payload_10k();
    assert!(
        payload.len() >= 10 * 1024,
        "payload must be >=10KB, got {}",
        payload.len()
    );
    // Leak to 'static for bench closure (avoids clone per iter when benchmarking)
    let payload_static: &'static str = Box::leak(payload.into_boxed_str());

    let mut group = c.benchmark_group("redact_10k");
    group.throughput(Throughput::Bytes(10 * 1024));

    // Primary: redact 10KB with secrets
    group.bench_function("redact_10k_with_secrets", |b| {
        b.iter(|| {
            let (masked, findings) = redactor.redact(std::hint::black_box(payload_static));
            std::hint::black_box((masked, findings));
        });
    });

    // Global (OnceLock) path — same implementation but via global singleton
    group.bench_function("redact_10k_global", |b| {
        b.iter(|| {
            let (masked, findings) = algo_redact::redact(std::hint::black_box(payload_static));
            std::hint::black_box((masked, findings));
        });
    });

    // Throughput batch: 100 redacts
    group.bench_function("throughput_100x10k", |b| {
        b.iter(|| {
            for _ in 0..100 {
                std::hint::black_box(redactor.redact(std::hint::black_box(payload_static)));
            }
        });
    });

    group.finish();

    // Sanity check: <500us/10KB budget from redact design
    let start = std::time::Instant::now();
    let iters = 1000usize;
    for _ in 0..iters {
        std::hint::black_box(redactor.redact(std::hint::black_box(payload_static)));
    }
    let avg_ns = start.elapsed().as_nanos() as f64 / iters as f64;
    let budget_ns = 500_000.0; // 500us
    if avg_ns > budget_ns {
        eprintln!(
            "[redact_10k] WARNING: sampled avg ~{:.0} ns ({:.1} us) exceeds 500us/10KB budget",
            avg_ns,
            avg_ns / 1000.0
        );
    } else {
        eprintln!(
            "[redact_10k] sampled avg ~{:.0} ns ({:.1} us) within 500us/10KB budget (payload 10KB)",
            avg_ns,
            avg_ns / 1000.0
        );
    }
    eprintln!("[redact_10k] baseline: cargo bench -p algo-redact --bench redact_10k -- --save-baseline p1-exit");
}

criterion_group!(benches, bench_redact_10k);
criterion_main!(benches);
