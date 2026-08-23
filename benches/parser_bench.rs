use criterion::{criterion_group, criterion_main, Criterion};
use vt100::Parser;

/// Build a representative ANSI stream: many colored lines (like `ls`/`top`
/// output). This is the kind of data the PTY reader feeds to the parser.
fn make_stream(lines: usize) -> Vec<u8> {
    let mut s = Vec::with_capacity(lines * 32);
    for i in 0..lines {
        let line = format!("\x1b[38;5;{}mfile_{:06}\x1b[0m\r\n", i % 256, i);
        s.extend_from_slice(line.as_bytes());
    }
    s
}

fn bench_parse(c: &mut Criterion) {
    let stream = make_stream(20_000);
    let mut parser = Parser::new(24, 80, 0);

    c.bench_function("parse_20k_colored_lines", |b| {
        b.iter(|| {
            parser.process(&stream);
        });
    });

    // A realistic burst: a smaller chunk like a single `ls` page, parsed many
    // times to measure per-chunk cost (this runs on the GPUI background thread).
    let chunk = make_stream(50);
    c.bench_function("parse_50_line_chunk", |b| {
        b.iter(|| {
            parser.process(&chunk);
        });
    });

    c.bench_function("resize_screen", |b| {
        b.iter(|| {
            parser.screen_mut().set_size(40, 120);
            parser.screen_mut().set_size(24, 80);
        });
    });
}

criterion_group!(benches, bench_parse);
criterion_main!(benches);
