// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use libprism::temp::{Allocatable, BlockAllocator};

#[repr(C)]
struct BenchBlock {
    id: u64,
    data: [u8; 1024],
}

unsafe impl Allocatable for BenchBlock {}

pub fn block_benchmark(c: &mut Criterion) {
    {
        let mut alloc = BlockAllocator::<BenchBlock>::with_capacity("block-bench", "", 4096)
            .expect("Failed to open database.");
        c.bench_function("block-no-cap", |b| {
            b.iter(|| {
                let block = alloc.allocate();
                black_box(block);
            })
        });
        println!("Len: {}\nCap: {}", alloc.len(), alloc.cap())
    }
    std::fs::remove_file("block-bench.db").expect("failed to remove benchmark.db");
    std::fs::remove_file("block-bench.map").expect("failed to remove benchmark.db.index");
}

criterion_group!(benches, block_benchmark);
criterion_main!(benches);
