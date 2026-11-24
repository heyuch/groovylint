# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial Cargo workspace with `core`, `syntax`, `analysis`, `config`, `rules`,
  and `cli` crates.
- Newline-aware Groovy lexer and recursive-descent parser for common syntax.
- Capability-based rule contracts (`TokenRule`, `AstRule`, `FileRule`) and a
  single shared AST traversal with node-interest dispatch.
- CodeNarc XML ruleset loading and vendored CodeNarc 4.0.0 `basic.xml` default.
- All 44 CodeNarc 4.0.0 `basic` rules, including the empty-block/declaration
  rules, conditional/comparison rules, API-usage rules, finally rules, and
  duplicate/cast/design rules.
- Token rule: `UnnecessarySemicolon`.
- One minimal Groovy fixture per rule and a table-driven end-to-end CLI test.
- Parallel file processing with deterministic diagnostics.
- Text and JSON output, source snippets, and lint exit codes.
- Parser, rule, config, and CLI integration tests.
- First differential validation against CodeNarc 4.0.0: the seven implemented
  `basic` rules match CodeNarc on rule name, line, and message for the bundled
  fixture.
- Gradle plugin (`io.github.heyuch.groovylint`) with per-source-set tasks,
  embedded macOS arm64 / Linux x86_64 native binaries, `check` integration, and
  configuration-cache support.
- Maven Central and Gradle Plugin Portal publication configuration plus a
  tag-driven GitHub Actions release pipeline.
- Criterion microbenchmarks and an end-to-end CodeNarc comparison harness.
- First public-corpus validation: 734 Apache Groovy files and 479 CodeNarc source
  files, with zero normalized finding differences; local median speedups of
  roughly 239x and 363x respectively on the staged corpora.
- Pinned Rust toolchain, `rustfmt`, Clippy, workspace lints, editor settings, CI,
  and Dependabot.
- Architecture notes covering the performance model, parser options, CodeNarc
  compatibility, and benchmark plan.
