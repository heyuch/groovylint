# groovylint

Static analysis for Apache Groovy, with CodeNarc-compatible rules and a path
toward deeper semantic checks such as nullness, type inference, and control-flow
analysis.

> [!IMPORTANT]
> **Status: first vertical slice.** The project now has a working lexer/parser
> subset, a layered rule engine, CodeNarc XML loading, parallel file processing,
> and basic CodeNarc rules. It is not yet a full Groovy parser or a complete
> CodeNarc replacement. Unsupported CodeNarc rules are reported and skipped, not
> silently ignored.

## What works today

- Newline-aware Groovy lexer and a recursive-descent parser for common syntax:
  classes, methods, fields, scripts, `if`/`else`, `while`, classic and enhanced
  `for`, `try`/`catch`/`finally`, closures, command-style calls, and common
  expressions.
- A single shared AST traversal with per-node interest masks. Rules implement
  capability interfaces:
  - `TokenRule` consumes the token stream;
  - `AstRule` receives enter/exit events from the shared traversal;
  - `FileRule` is reserved for deferred/file-level analyses.
- CodeNarc XML ruleset loading, including the vendored CodeNarc 4.0.0
  `rulesets/basic.xml` default.
- Parallel file discovery and linting via Rayon, with deterministic output.
- Text and JSON reporters, source snippets, and exit codes.
- Seven CodeNarc `basic` rules plus `UnnecessarySemicolon`.

### Supported rules

| Rule | CodeNarc category | Layer |
| --- | --- | --- |
| `EmptyIfStatement` | `basic` | AST |
| `EmptyElseBlock` | `basic` | AST |
| `EmptyWhileStatement` | `basic` | AST |
| `EmptyForStatement` | `basic` | AST |
| `EmptyTryBlock` | `basic` | AST |
| `EmptyCatchBlock` | `basic` | AST |
| `EmptyFinallyBlock` | `basic` | AST |
| `UnnecessarySemicolon` | `unnecessary` | token |

The default ruleset contains 44 CodeNarc `basic` rules. The CLI runs the seven
supported rules from that list and prints a note with the number skipped. A
custom CodeNarc XML ruleset can enable `UnnecessarySemicolon` or narrow the set.

## Quick start

Prerequisites: Rustup. The pinned toolchain in `rust-toolchain.toml` is installed
automatically on first use.

```bash
# Show CLI help
cargo run -p groovylint -- --help

# Analyze the bundled fixture
cargo run -p groovylint -- check \
  crates/groovylint-cli/tests/fixtures/basic.groovy

# Machine-readable output
cargo run -p groovylint -- check \
  --format json \
  crates/groovylint-cli/tests/fixtures/basic.groovy

# Focused ruleset (enables the token rule)
cargo run -p groovylint -- check \
  --config crates/groovylint-cli/tests/fixtures/strict.xml \
  crates/groovylint-cli/tests/fixtures/basic.groovy

# Parallelism is explicit; output remains deterministic
cargo run -p groovylint -- check --threads 4 .
```

Exit codes:

- `0`: no diagnostics;
- `1`: diagnostics found, or a source file failed to parse;
- `2`: usage, configuration, or I/O error.

## Architecture

```text
file discovery
    -> read + lex + parse                     (parallel across files)
    -> one AST traversal
         -> dispatch AstRule events
         -> collect diagnostics
    -> TokenRule phase
    -> FileRule phase (reserved)
    -> normalize + sort + report
```

Rules do not implement their own AST walks. Each `AstRule` declares a
`NodeMask`, and the engine only invokes interested rules for each node. Rule
instances are created per file, so traversal state is not shared across threads.

The CodeNarc compatibility surface currently includes rule names and CodeNarc
class-name lookup. It does not yet implement CodeNarc base properties, rule
properties, suppressions, or `<ruleset-ref>`.

## Parser scope and limitations

The parser intentionally targets common Groovy rather than full language
coverage. Known gaps include:

- `switch`, `do`/`while`, and labeled statements;
- enum constants (skipped rather than represented);
- GString interpolation details and slashy/dollar-slashy strings;
- multi-variable declarations and some command-chain forms;
- annotations are consumed but not represented in the AST;
- no parser error recovery yet: the first syntax error becomes a `ParseError`
  diagnostic for that file.

These are expected to shrink as rules demand more syntax. The parser is
self-developed; see [the architecture notes](docs/ARCHITECTURE.md) for the
rationale and the conformance plan.

## Workspace layout

```text
crates/
  groovylint-core/       spans, source files, diagnostics
  groovylint-syntax/     lexer, AST, parser
  groovylint-analysis/   CFG/data-flow/type foundation (placeholder)
  groovylint-config/     CodeNarc XML ruleset loading
  groovylint-rules/      rule contracts, registry, traversal, engine, rules
  groovylint-cli/        binary, discovery, parallel fan-out, reporters
docs/
  ARCHITECTURE.md        design, parser rationale, compatibility and benchmark plan
```

## Quality gates

```bash
cargo fmt --all
cargo fmt-check
cargo lint
cargo test-all
cargo doc-all
```

- Rust `1.100.0` is pinned in `rust-toolchain.toml`.
- Workspace-level Clippy lints (`all` + `pedantic`) treat warnings as errors in
  CI.
- `.editorconfig` and VS Code settings enable format-on-save and Clippy.
- GitHub Actions runs formatting, Clippy, tests, and documentation checks.
- Dependabot keeps Cargo and GitHub Actions dependencies current.

## Next steps

1. Add a CodeNarc differential test harness for the implemented rules.
2. Add `UnusedVariable` to exercise scope/symbol facts and deferred file rules.
3. Add a benchmark crate (Criterion/`iai-callgrind`) and a pinned CodeNarc
   comparison on the same corpus.
4. Expand parser coverage based on real Groovy workloads.
5. Implement CodeNarc base-rule properties and suppression behavior.
6. Then introduce CFG/data-flow and nullness analysis.

## Documentation

- [Architecture and open questions](docs/ARCHITECTURE.md)
- [CodeNarc compatibility notes](docs/COMPATIBILITY.md)
- [Contributing](CONTRIBUTING.md)
- [Changelog](CHANGELOG.md)

## License

MIT. The vendored CodeNarc basic ruleset is Apache-2.0; see
`crates/groovylint-config/rulesets/basic.xml` for attribution.
