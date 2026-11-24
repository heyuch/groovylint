# Autofix design

Status: design proposal, 2026-11-25.

Autofix means producing a concrete source edit for a finding, showing it as a
suggestion, and optionally applying safe edits. It is not a formatter and it is
not permission for a rule to rewrite arbitrary source.

## Principles

1. **Rules propose edits; the engine applies them.** A rule never writes files.
2. **Text edits are explicit and minimal.** Every edit has a byte span and a
   replacement string.
3. **Safe by default.** Only semantics-preserving fixes run under `--fix`.
4. **Reparse and revalidate.** A fixed file is parsed again; if the fix creates a
   syntax error, the file is restored.
5. **Deterministic and idempotent.** The same input produces the same edits.
   Applying fixes repeatedly reaches a fixed point or stops with a diagnostic.
6. **Preserve comments and untouched formatting.** Fixes use CST/trivia spans
   when available; they never reprint the whole file.
7. **Not every finding is fixable.** Suggestions are still valuable; architecture
   and nullness findings are normally not auto-fixable.

## Edit model

Add to the core diagnostic model:

```rust
pub struct TextEdit {
    pub span: Span,
    pub replacement: String,
}

pub enum FixSafety {
    /// Preserves behavior and intent; eligible for `--fix`.
    Safe,
    /// Usually correct but may change behavior; requires `--fix-unsafe`.
    Unsafe,
}

pub struct Fix {
    pub title: String,
    pub safety: FixSafety,
    pub edits: Vec<TextEdit>,
}
```

`Diagnostic` gains an optional list of fixes. JSON output should expose them
with byte offsets and line/column positions. A fix may contain multiple edits,
for example replacing a call and deleting an unused import.

Rules use a sink helper:

```rust
sink.report_with_fix(
    span,
    "message",
    Fix::safe("Remove the redundant semicolon")
        .delete(semicolon_span),
);
```

The engine validates that all spans are within the file and that edits do not
overlap unless they belong to the same fix.

## Application algorithm

For each file:

1. Lint the file and collect diagnostics with fixes.
2. Select fixes according to CLI policy:
   - `--fix`: safe fixes only;
   - `--fix-unsafe`: safe and unsafe fixes;
   - `--fix-dry-run` / `--diff`: no write, print a unified diff.
3. Sort selected edits by descending start offset.
4. Reject overlapping edits deterministically (stable order by rule ID, fix
   title, start, end).
5. Apply edits to an in-memory buffer.
6. Reparse the result. If parsing fails, discard all edits for that file and
   report the original fix as rejected.
7. Write via a temporary file and atomic rename; preserve permissions and line
   endings.
8. Re-lint the fixed file.
9. Repeat up to a bounded number of passes (initially 10). Stop when no edits
   are produced, when a content hash repeats, or when the pass limit is reached.

`--fix` should not hide remaining findings. The command prints unfixed
diagnostics and exits non-zero if any remain, unless `--ignore-failures` is
used.

## Fix safety policy

A fix is `Safe` only when all of these hold:

- the edit does not change runtime semantics;
- comments and shebangs are preserved;
- the edit does not depend on unresolved dynamic dispatch;
- the fixed file reparses;
- the rule no longer reports the same finding at the same location;
- the change is local to the finding, not a whole-file reformat.

Examples:

| Rule | Fix | Safety |
|---|---|---|
| `UnnecessarySemicolon` | delete `;` | Safe |
| `TrailingWhitespace` | delete trailing spaces | Safe |
| `UnusedImport` | delete the import and its attached trivia | Safe, with comment handling |
| `UnnecessaryPublicModifier` | delete `public` | Safe |
| spacing rules | insert/delete whitespace | Safe if token boundaries are unambiguous |
| `MissingBlankLine...` | insert a blank line | Safe if the configured style is known |
| `UnnecessaryGString` | replace quotes | Suggestion/Unsafe |
| `UnnecessaryReturnKeyword` | remove `return` | Unsafe; closure semantics can differ |
| `ExplicitCallToPlusMethod` | `a.plus(b)` → `a + b` | Suggestion; operator overload edge cases |
| Architecture rules | dependency edits | Suggestion only; never automatic |
| Nullness rules | add guard/assert | Suggestion only; never automatic |

A rule's default fixability is metadata:

```rust
pub enum Fixability {
    None,
    Suggestion,
    Safe,
    Unsafe,
}
```

The actual `Fix` carries the final safety. Metadata powers `rules --coverage`
and documentation.

## CLI UX

```bash
# read-only
groovylint check .

# apply safe fixes, then report remaining findings
groovylint check --fix .

# show the diff without writing
groovylint check --fix --dry-run .

# include unsafe fixes explicitly
groovylint check --fix --unsafe .

# machine-readable edits
groovylint check --format json .
```

Text output should show a short fix summary and, in dry-run mode, a unified
diff. JSON output adds a `fixes` array to each diagnostic; the schema version
must be bumped when this is added.

## Gradle UX

Do not mutate files during `check`. Add explicit tasks:

- `groovylintFixMain`
- `groovylintFixTest`
- `groovylintFix` (aggregate)

These tasks are opt-in and are not dependencies of `check`. A separate
`--dry-run` equivalent can be provided through a task property or docs, but the
first version can expose only the CLI diff mode.

## Rule API impact

- `DiagnosticSink` gains `report_with_fix` and possibly
  `report_with_suggestions`.
- `Diagnostic` gains `fixes: Vec<Fix>` (or a single optional `Fix` plus
  suggestions).
- `RuleMeta` gains `fixability`.
- The reporter must render fix titles and, optionally, edit ranges.
- Rule tests should assert both diagnostics and fixes, including a "fix then
  re-lint" test.

## Initial fix set

Start with fixes that are local and mechanically verifiable:

1. `UnnecessarySemicolon`
2. `TrailingWhitespace`
3. `UnusedImport` (only after trivia/comment handling exists)
4. `UnnecessaryPublicModifier`
5. unambiguous spacing rules
6. `MissingBlankLine...` where the configured style is explicit

Add rules incrementally. A rule without a safe fix can still expose a
suggestion.

## Testing

- Unit tests for edit ordering, overlap rejection, CRLF, BOM, and Unicode.
- Fix-then-parse tests for every safe fix.
- Fix-then-lint idempotence tests.
- Golden diffs for representative files.
- Corpus test: `--fix --dry-run` must never produce an invalid file; a full
  corpus fix run must terminate.
- Property tests for edit application (applying an empty edit list is identity;
  applying sorted non-overlapping edits equals a reference implementation).

## Dependencies and sequencing

Autofix depends on:

- reliable spans for every AST/token node;
- comment/trivia preservation for deletions near comments;
- the lossless syntax work in [`SYNTAX.md`](SYNTAX.md);
- a stable diagnostic schema.

Therefore the roadmap places autofix after the lossless syntax foundation and
before the architecture/nullness work. Simple token-level fixes can be
prototyped earlier, but they should not ship without comment-preservation tests.
