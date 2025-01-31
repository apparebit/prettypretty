use criterion::{criterion_group, criterion_main, Criterion};
use prettypretty::theme::Theme;
use prettytty::opt::Options;
use prettytty::Connection;

pub fn run_benchmarks(c: &mut Criterion) {
    let options = Options::with_log();
    let tty =
        Connection::with_options(options).expect("need terminal connection to run benchmarks");

    let mut group = c.benchmark_group("theme-query");
    group.sample_size(10);

    group.bench_function("1-loop", |b| b.iter(|| Theme::query1(&tty)));

    group.bench_function("2-loops", |b| b.iter(|| Theme::query2(&tty)));

    group.bench_function("3-loops", |b| b.iter(|| Theme::query3(&tty)));

    group.finish();
}

criterion_group!(benches, run_benchmarks);
criterion_main!(benches);
