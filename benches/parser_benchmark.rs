use criterion::{black_box, criterion_group, criterion_main, Criterion};

const PRELUDE: &str = include_str!("./lumesh-prelude");

pub fn tokenize_benchmark(c: &mut Criterion) {
    c.bench_function("tokenize prelude", |b| {
        b.iter(|| lumesh::tokenize(black_box(PRELUDE)))
    });
}

pub fn parse_benchmark(c: &mut Criterion) {
    c.bench_function("parse prelude", |b| {
        b.iter(|| lumesh::parse_script(black_box(PRELUDE)))
    });
}

criterion_group!(benches, tokenize_benchmark, parse_benchmark);
criterion_main!(benches);
