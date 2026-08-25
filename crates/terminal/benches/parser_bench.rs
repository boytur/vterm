use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::term::{Config, Term};
use alacritty_terminal::vte::ansi::{Processor, StdSyncHandler};

#[derive(Default)]
struct BenchProxy;

impl EventListener for BenchProxy {
    fn send_event(&self, _event: Event) {}
}

/// Grid bounds for the bench; keeps `Term::new`/`resize` terse.
struct BenchBounds {
    lines: usize,
    columns: usize,
}

impl Dimensions for BenchBounds {
    fn total_lines(&self) -> usize {
        self.lines
    }
    fn screen_lines(&self) -> usize {
        self.lines
    }
    fn columns(&self) -> usize {
        self.columns
    }
}

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
    let mut term = Term::new(
        Config {
            scrolling_history: 0,
            ..Default::default()
        },
        &BenchBounds {
            lines: 24,
            columns: 80,
        },
        BenchProxy,
    );
    let mut processor = Processor::<StdSyncHandler>::new();

    c.bench_function("parse_20k_colored_lines", |b| {
        b.iter(|| {
            processor.advance(&mut term, black_box(&stream));
        });
    });

    // A realistic burst: a smaller chunk like a single `ls` page, parsed many
    // times to measure per-chunk cost (this runs on the GPUI background thread).
    let chunk = make_stream(50);
    c.bench_function("parse_50_line_chunk", |b| {
        b.iter(|| {
            processor.advance(&mut term, black_box(&chunk));
        });
    });

    c.bench_function("resize_screen", |b| {
        b.iter(|| {
            term.resize(BenchBounds {
                lines: 40,
                columns: 120,
            });
            term.resize(BenchBounds {
                lines: 24,
                columns: 80,
            });
        });
    });
}

criterion_group!(benches, bench_parse);
criterion_main!(benches);
