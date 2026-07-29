//! Money 金额运算基准测试
//!
//! 测试 rust_decimal::Decimal 的加减乘除性能。
//! 对比 f64 运算作为基准线。

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rust_decimal_macros::dec;

fn bench_decimal_add(c: &mut Criterion) {
    let a = dec!(9999.99);
    let b = dec!(0.01);

    c.bench_function("decimal_add", |b_iter| {
        b_iter.iter(|| {
            black_box(a + b);
        })
    });

    // 对比 f64
    let af = 9999.99f64;
    let bf = 0.01f64;

    c.bench_function("f64_add", |b_iter| {
        b_iter.iter(|| {
            black_box(af + bf);
        })
    });
}

fn bench_decimal_mul(c: &mut Criterion) {
    let amount = dec!(10000.00);
    let rate = dec!(0.006);

    c.bench_function("decimal_mul_fee", |b_iter| {
        b_iter.iter(|| {
            black_box(amount * rate);
        })
    });

    // 对比 f64
    let af = 10000.00f64;
    let rf = 0.006f64;

    c.bench_function("f64_mul_fee", |b_iter| {
        b_iter.iter(|| {
            black_box(af * rf);
        })
    });
}

criterion_group!(benches, bench_decimal_add, bench_decimal_mul);
criterion_main!(benches);
