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
- Basic rules: `EmptyIfStatement`, `EmptyElseBlock`, `EmptyWhileStatement`,
  `EmptyForStatement`, `EmptyTryBlock`, `EmptyCatchBlock`,
  `EmptyFinallyBlock`.
- Token rule: `UnnecessarySemicolon`.
- Parallel file processing with deterministic diagnostics.
- Text and JSON output, source snippets, and lint exit codes.
- Parser, rule, config, and CLI integration tests.
- First differential validation against CodeNarc 4.0.0: the seven implemented
  `basic` rules match CodeNarc on rule name, line, and message for the bundled
  fixture.
- Pinned Rust toolchain, `rustfmt`, Clippy, workspace lints, editor settings, CI,
  and Dependabot.
- Architecture notes covering the performance model, parser options, CodeNarc
  compatibility, and benchmark plan.
