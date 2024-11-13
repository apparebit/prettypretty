use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use prettypretty::term::terminal;
use prettypretty::theme;

pub fn run_benchmarks(c: &mut Criterion) {
    unsafe { terminal().connect() }.unwrap();

    // Compare the query functions without the overhead of object creation.
    let mut group = c.benchmark_group("theme-query");
    group.sample_size(10);

    group.bench_function(
        "1-loop",
        | b | {
            b.iter_batched(
                || theme::prepare(false).expect("access to terminal"),
                |(mut tty, mut scanner, mut theme)| {
                    theme::query1(&mut tty, &mut scanner, &mut theme)
                },
                BatchSize::PerIteration,
            )
        },
    );

    group.bench_function(
        "2-loops",
        | b | {
            b.iter_batched(
                || theme::prepare(false).expect("access to terminal"),
                |(mut tty, mut scanner, mut theme)| {
                    theme::query2(&mut tty, &mut scanner, &mut theme)
                },
                BatchSize::PerIteration,
            )
        },
    );

    group.bench_function(
        "3-loops",
        | b | {
            b.iter_batched(
                || theme::prepare(false).expect("access to terminal"),
                |(mut tty, mut scanner, mut theme)| {
                    theme::query3(&mut tty, &mut scanner, &mut theme)
                },
                BatchSize::PerIteration,
            )
        },
    );

    group.finish();

    terminal().disconnect();
}

criterion_group!(benches, run_benchmarks);
criterion_main!(benches);
