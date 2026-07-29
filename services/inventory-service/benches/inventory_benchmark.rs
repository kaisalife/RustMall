//! 库存操作基准测试
//!
//! 测试库存领域模型的内存操作性能。
//! 预扣减/扣减/释放已改为原子 SQL，不再在域模型中测试。

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use inventory_service::domain::inventory::Inventory;

fn bench_add_stock(c: &mut Criterion) {
    c.bench_function("add_stock", |b| {
        b.iter_batched(
            || Inventory::new(1, 1000000),
            |mut inv| {
                inv.add_stock(100).unwrap();
                black_box(());
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_available_quantity(c: &mut Criterion) {
    c.bench_function("available_quantity", |b| {
        b.iter_batched(
            || Inventory::new(1, 1000000),
            |inv| {
                black_box(inv.available_quantity());
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, bench_add_stock, bench_available_quantity);
criterion_main!(benches);
