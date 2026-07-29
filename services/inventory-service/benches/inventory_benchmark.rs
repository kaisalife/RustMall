//! 库存操作基准测试
//!
//! 测试库存领域模型的扣减、预留、释放性能。
//! 纯内存操作，不涉及数据库 IO。

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use inventory_service::domain::inventory::Inventory;

fn bench_deduct_stock(c: &mut Criterion) {
    c.bench_function("deduct_stock_success", |b| {
        b.iter_batched(
            || Inventory::new(1, 1000000),
            |mut inv| {
                let _: () = inv.deduct_stock(100).unwrap();
                black_box(());
            },
            criterion::BatchSize::SmallInput,
        )
    });

    c.bench_function("deduct_stock_insufficient", |b| {
        b.iter_batched(
            || Inventory::new(1, 50),
            |mut inv| {
                black_box(inv.deduct_stock(100).is_err());
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_reserve_release(c: &mut Criterion) {
    c.bench_function("reserve_and_release", |b| {
        b.iter_batched(
            || {
                let mut inv = Inventory::new(1, 1000000);
                inv.reserve_stock(500).unwrap();
                inv
            },
            |mut inv| {
                let _: () = inv.release_reserved(500).unwrap();
                black_box(());
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, bench_deduct_stock, bench_reserve_release);
criterion_main!(benches);
