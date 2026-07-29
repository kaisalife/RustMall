//! 加密操作基准测试
//!
//! 测试 bcrypt 密码哈希和 JWT 生成/验证的性能。

use common::{crypto, Claims};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_password_hash(c: &mut Criterion) {
    let password = "TestPassword123";

    c.bench_function("hash_password", |b| {
        b.iter(|| {
            black_box(crypto::hash_password(black_box(password)).unwrap());
        })
    });

    // 预先生成哈希用于验证测试
    let hash = crypto::hash_password(password).unwrap();

    c.bench_function("verify_password_correct", |b| {
        b.iter(|| {
            black_box(crypto::verify_password(black_box(password), black_box(&hash)).unwrap());
        })
    });

    c.bench_function("verify_password_wrong", |b| {
        b.iter(|| {
            black_box(
                crypto::verify_password(black_box("WrongPassword"), black_box(&hash)).is_err(),
            );
        })
    });
}

fn bench_jwt(c: &mut Criterion) {
    let secret = "benchmark-secret-key";
    let claims = Claims::new(1, "bench@example.com".to_string(), 24, "user".to_string());
    let token = crypto::generate_jwt(&claims, secret).unwrap();

    c.bench_function("generate_jwt", |b| {
        b.iter(|| {
            black_box(crypto::generate_jwt(black_box(&claims), black_box(secret)).unwrap());
        })
    });

    c.bench_function("validate_jwt", |b| {
        b.iter(|| {
            black_box(crypto::validate_jwt(black_box(&token), black_box(secret)).unwrap());
        })
    });
}

criterion_group!(benches, bench_password_hash, bench_jwt);
criterion_main!(benches);
