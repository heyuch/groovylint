use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use groovylint_core::SourceFile;
use groovylint_rules::{Engine, RuleRegistry};
use groovylint_syntax::parser::parse;

const FIXTURE: &str = include_str!("../tests/fixtures/rules/EmptyIfStatement.groovy");

fn source() -> String {
    FIXTURE.repeat(200)
}

fn bench_parse(criterion: &mut Criterion) {
    let source = source();
    criterion.bench_function("parse_200_groovy_units", |bencher| {
        bencher.iter(|| {
            for _ in 0..200 {
                let parsed = parse(black_box(&source)).expect("fixture parses");
                black_box(parsed.script.items.len());
            }
        });
    });
}

fn bench_lint(criterion: &mut Criterion) {
    let source = source();
    let parsed = parse(&source).expect("fixture parses");
    let file = SourceFile::new("Benchmark.groovy", source);
    let registry = RuleRegistry::builtin();
    let engine = Engine::recommended(&registry);

    criterion.bench_function("lint_basic_rules", |bencher| {
        bencher.iter(|| {
            let diagnostics = engine.lint(black_box(&file), black_box(&parsed));
            black_box(diagnostics.len());
        });
    });
}

criterion_group!(benches, bench_parse, bench_lint);
criterion_main!(benches);
