//! 雪花算法 ID 生成基准测试
//!
//! 测试单线程批量生成 ID 的吞吐量。

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use common::SnowflakeIdGenerator;

fn bench_generate_id(c: &mut Criterion) {
    let generator = SnowflakeIdGenerator::new(1).unwrap();

    c.bench_function("generate_single_id", |b| {
        b.iter(|| {
            black_box(generator.generate().unwrap());
        })
    });

    // 批量生成测试
    let mut group = c.benchmark_group("generate_batch_ids");
    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                for _ in 0..size {
                    black_box(generator.generate().unwrap());
                }
            })
        });
    }
    group.finish();
}

criterion_group!(benches, bench_generate_id);
criterion_main!(benches);
