use algo_redact::Redactor;
use std::time::Instant;

fn main() {
    let r = Redactor::new();
    let input = "a".repeat(10 * 1024);
    let start = Instant::now();
    for _ in 0..1000 {
        let _ = r.redact(&input);
    }
    let elapsed = start.elapsed();
    println!("1000x 10KB: {:?} per iteration {:?}", elapsed, elapsed / 1000);
    let single = Instant::now();
    let _ = r.redact(&input);
    println!("single 10KB: {:?}", single.elapsed());
    // Also test with a secret inside
    let with_secret = format!("{} AKIAIOSFODNN7EXAMPLE {}", "b".repeat(5*1024), "c".repeat(5*1024));
    let start2 = Instant::now();
    let (masked, findings) = r.redact(&with_secret);
    let elapsed2 = start2.elapsed();
    println!("10KB with secret: {:?} findings {} masked len {}", elapsed2, findings.len(), masked.len());
}
