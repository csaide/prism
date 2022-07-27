// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use libprism::temp::Log;

pub fn log_benchmark(c: &mut Criterion) {
    {
        let mut log = Log::new("benchmark.db").expect("Failed to open database.");
        c.bench_function("log-no-cap", |b| b.iter(|| log.push(black_box(1u64))));
        println!("Len: {}\nCap: {}", log.len(), log.cap())
    }
    std::fs::remove_file("benchmark.db").expect("failed to remove benchmark.db");
    std::fs::remove_file("benchmark.db.index").expect("failed to remove benchmark.db.index");
}

pub fn log_with_cap_benchmark(c: &mut Criterion) {
    {
        let mut log =
            Log::with_capacity("with-cap.db", 2147483648).expect("Failed to open database.");
        c.bench_function("log-with-cap", |b| b.iter(|| log.push(black_box(1u64))));
        println!("Len: {}\nCap: {}", log.len(), log.cap())
    }
    std::fs::remove_file("with-cap.db").expect("failed to remove with-cap.db");
    std::fs::remove_file("with-cap.db.index").expect("failed to remove with-cap.db.index");
}

criterion_group!(benches, log_benchmark, log_with_cap_benchmark);
criterion_main!(benches);
