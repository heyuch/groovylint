# Syntax front end: AST, CST, and formatter readiness

Status: design proposal, 2026-11-25.

The syntax layer must serve three consumers without coupling them together:

1. rules and semantic analysis, which want a compact semantic AST;
2. autofix, which needs reliable source spans and token/trivia boundaries;
3. a future formatter, which needs a lossless representation of the source.

The formatter is not part of the current milestone. The syntax layer should be
ready for it before formatter work starts.

## Current state

`groovylint-syntax` currently depends only on `groovylint-core`. That crate
boundary is correct and should stay: syntax must not depend on rules, config,
reporting, or CLI.

The current implementation is semantic-AST-first:

- the lexer keeps newline tokens because Groovy statement termination is
  newline-sensitive;
- line comments, block comments, spaces, and tabs are skipped;
- AST nodes carry byte spans;
- the parser returns a `Script` and the first parse error;
- there is no concrete syntax tree and no comment/trivia preservation.

This is sufficient for the current rules, but not for a formatter or for fixes
that must preserve comments around deleted or replaced nodes. For example,
deleting an unused import must not delete a comment that belongs to it, and
formatting a block must not lose a trailing comment.

## Goals

- Keep the semantic AST stable and independent of formatting concerns.
- Add a lossless token/trivia layer.
- Add a lossless CST that can reproduce the original source exactly.
- Lower the semantic AST from the same parse, without requiring the CST.
- Preserve comments, shebangs, line endings, string delimiters, and literal
  spelling in the lossless representation.
- Make parser errors collectable and recoverable enough to build a partial CST.
- Keep syntax performance suitable for the existing parser/lint benchmarks.

## Non-goals

- Building the formatter in this milestone.
- Replacing the semantic AST with a rowan tree.
- Putting formatting policy into the parser.
- Supporting invalid Groovy as a stable formatter input.
- Exact byte-for-byte preservation after formatting; only the lossless CST
  guarantees that before formatting.

## Proposed representation

### Lossless token stream

Introduce a trivia model instead of discarding comments and whitespace:

```rust
pub enum TriviaKind {
    Whitespace,
    LineComment,
    BlockComment,
    Shebang,
}

pub struct Trivia {
    pub kind: TriviaKind,
    pub span: Span,
}

pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    pub leading_trivia: Vec<Trivia>,
    pub trailing_trivia: Vec<Trivia>,
}
```

Alternative: keep a parallel `Vec<Trivia>` plus token indices. That avoids a
heap allocation per token and is preferable for performance. The exact memory
layout should be benchmarked.

Newlines are currently tokens. They should remain tokens for Groovy's
newline-sensitive grammar; whitespace/comment trivia is attached around them.

### CST

Add a concrete syntax tree where every token and every trivia item is present.
Each node has:

- a `SyntaxKind`;
- a byte span covering its tokens;
- child nodes and token indices;
- no semantic interpretation.

Candidate implementation:

- a small red/green tree similar to `rowan`;
- use `rowan` directly if its dependency and API fit the project;
- a custom event-based tree if the dependency is undesirable.

The parser should emit events rather than constructing the AST directly:
`StartNode`, `Token`, `FinishNode`, `Error`. A tree builder can construct the CST;
the semantic AST can be lowered from the same events or from the CST. The AST
must not contain CST nodes.

### Public API

Keep the existing `parse()` entry point for callers that only need the AST, and
add a lossless result:

```rust
pub struct ParsedFile {
    pub ast: Script,
    pub cst: SyntaxNode,
    pub errors: Vec<ParseError>,
}

pub fn parse_lossless(source: &str) -> ParsedFile;
```

`parse_lossless` should not fail on the first syntax error; it should collect
errors and return a partial CST where possible. Consumers that require valid
syntax (formatter, safe autofix) must check `errors.is_empty()`.

## AST independence rules

- `groovylint-syntax` must not depend on `groovylint-rules`,
  `groovylint-config`, `groovylint-analysis`, or the CLI.
- AST nodes describe Groovy structure and spans only. They do not contain rule
  names, severities, formatting decisions, or comment attachment policy.
- Parser errors use syntax-local error types, not `Diagnostic`.
- The AST remains the stable contract for rules and analysis. The CST is an
  additional representation, not a replacement.
- Every AST node must have a span that is usable for diagnostics. A formatter
  may use a larger CST span for a node, but must not rely on AST spans for
  comments.
- AST changes require syntax fixtures and corpus round-trip checks.

## Formatter readiness

Before implementing a formatter, require:

1. **Exact round-trip:** for every parseable corpus file,
   `cst.text() == original_source`.
2. **Comment preservation:** formatting never drops or duplicates comments or
   shebangs.
3. **Idempotence:** `format(format(source)) == format(source)` for the formatter
   test corpus.
4. **Token fidelity:** string/slashy/regex delimiters, numeric literal spelling,
   and line endings can be preserved or intentionally normalized.
5. **Error policy:** files with parse errors are not formatted.
6. **Incremental format:** formatting can be limited to a line range later; the
   first version may format whole files.

A future `groovylint-format` crate should depend only on `groovylint-syntax`. It
must not be called by rules to implement a fix; autofix uses explicit text edits
(see [`AUTOFIX.md`](AUTOFIX.md)).

## Testing

- Golden parser tests for every supported construct.
- Round-trip tests over the public and private corpora.
- Comment preservation tests, including comments before/after imports, fields,
  methods, blocks, and closing braces.
- Newline and CRLF tests.
- Unicode and byte-span tests.
- Fuzz tests: parser must not panic; lossless CST must either round-trip or
  report errors.
- Benchmarks for tokenization, AST-only parse, and CST parse.

## Migration plan

1. Add trivia-preserving tokenization behind a new API without changing existing
   `lex()` callers.
2. Add a lossless CST builder and round-trip tests.
3. Add `parse_lossless`; keep `parse()` as an AST-only fast path.
4. Move autofix to token/CST spans.
5. Build the formatter later in a separate crate.
