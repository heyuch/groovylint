# Incremental analysis and caching

Status: design proposal, 2026-11-25.

Performance work is split into two parts:

1. **Continuous measurement** from the first milestone: benchmarks, performance
   budgets, and regression checks.
2. **Incremental analysis and persistent caching** as a dedicated late milestone,
   after the rule, syntax, project-graph, and nullness semantics are stable.

Caching before semantics are stable would only make stale results faster.

## Goals

- Avoid re-analyzing unchanged files.
- Reuse file facts when only unrelated rules change.
- Keep project-level architecture rules correct when one file changes.
- Make Gradle `check` incremental and compatible with Gradle's build cache.
- Provide explicit cache controls and understandable cache statistics.
- Keep cached and uncached results identical.

## Non-goals

- A distributed/remote analysis cache in the first version.
- Caching a serialized AST/CST before profiling shows parse cost dominates.
- Using file modification time alone as a correctness key.
- Hiding stale results when the ruleset, analyzer, or plugin changes.
- Making `groovylintFix` cacheable; fix tasks mutate sources.

## Current state

- The CLI parses and lints every discovered file on every run.
- The engine is file-level and parallel; there is no project-level result cache.
- The Gradle plugin's lint task has no report output, so Gradle cannot perform
  normal up-to-date or build-cache checks for it.
- The benchmark suite already measures parser/linter cost, but not incremental
  hit rates or one-file-change latency.

## Cache layers

| Layer | Key inputs | Stored value | Invalidation |
|---|---|---|---|
| Per-rule file result | file content hash, rule ID, rule properties, parser version, analyzer version | diagnostics + spans | file/config/analyzer change |
| File facts | file content hash, facts schema version, resolver version | package/imports/classes/references/dependency edges | parser/facts change |
| Project graph | set of file-fact keys, module mapping, resolver version | symbol/dependency graph | any file/module/resolver change |
| Nullness summaries | method source hash, analysis version, callee summary keys | method/field summaries | method/callee/config change |
| Gradle task output | Gradle inputs + build script + plugin version | report output | Gradle build-cache rules |

Start with per-rule file results and file facts. Add nullness summaries only after
the nullness analysis has stable summaries and benchmarks show a need.

## Cache keys

A cache key must include every input that can change the result:

```text
analyzer_version
parser_version / facts_schema_version
rule_id
rule_properties (canonicalized)
severity / suppression configuration
file_content_hash
file_path or module-relative path (for relative resolution)
module_mapping_hash
classpath_hash (if type resolution or external summaries are used)
plugin_id / plugin_version / plugin_api_version
```

Rules:

- Hash **content**, not just mtime/size. Metadata can be used as a hint, not as
  proof of correctness.
- Canonicalize XML/TOML/JSON configuration before hashing; do not hash
  formatting-only differences or absolute temp paths.
- Include the analyzer build identity, not only the release version, so local
  development builds invalidate correctly.
- Include plugin hashes when plugin rules are enabled.
- Bump a cache-format version when serialized types change; old entries are
  discarded rather than migrated in place.
- Never reuse a finding across a changed file path if the rule depends on path
  or module mapping.

## Storage

Initial implementation:

- default cache root: user cache directory
  (`$XDG_CACHE_HOME/groovylint/<project-hash>` or the macOS equivalent);
- `--cache-dir` for CI and reproducible environments;
- sharded content-addressed files, one result per key;
- atomic temp-file + rename writes;
- checksum and schema version in each entry;
- corrupted entries are ignored and recomputed;
- `--no-cache` bypasses reads and writes;
- `--cache-stats` prints lookups, hits, misses, evictions, and bytes.

If the number of entries or concurrent writers becomes a problem, move to a
single embedded database (for example SQLite) after benchmarking. Do not start
with a database before measuring.

Cache size and eviction:

- a configurable maximum size;
- least-recently-used eviction;
- `--cache-prune` and a documented manual cleanup path;
- cache directory permissions restricted to the user.

## Incremental algorithm

Correctness-first version:

1. Discover files and compute content hashes.
2. Look up per-rule results for each file.
3. Parse/lint only cache misses.
4. Look up file facts; recompute only for changed files.
5. Rebuild or update the project graph from cached + new facts.
6. Re-run project rules (initially all of them) against the graph.
7. Emit a complete, deterministically sorted report.

Optimized version:

1. Use file-fact deltas to update graph edges.
2. Determine impacted project rules and impacted nullness methods.
3. Recompute only those rules/methods.
4. Reuse diagnostics for untouched rules.
5. Merge and sort the final report.

The initial version may re-run all architecture rules after a file change. That is
acceptable if the graph build is cheap; optimize only after measuring.

## Project rules and nullness

Project-level analyses need extra care:

- **Architecture rules:** a changed file can add/remove classes, imports, and
  edges. Rebuild the graph from cached facts first; re-run all architecture
  rules. Later, compute an impacted rule set from changed node/edge types.
- **Nullness:** a changed method can affect callers and callees. Start by
  re-analyzing changed methods and their direct callers, then expand to a
  conservative reverse-call-graph closure. Do not cache a summary whose callee
  summaries changed.
- **Deleted files:** remove their facts and all incident edges; re-run rules that
  can be affected by the deletion.
- **Renamed files:** treat as delete + add unless a content-identical move cannot
  affect path-sensitive rules. Correctness first.

## CLI UX

Proposed controls:

```bash
groovylint check .                    # cache on by default, if enabled
groovylint check --no-cache .
groovylint check --cache-dir .cache/groovylint .
groovylint check --cache-stats .
groovylint check --changed-files changed.txt .
```

`--changed-files` is an optimization hint, not a correctness contract: the tool
must still account for deleted files and project-rule dependencies. A future
watch mode can reuse the same incremental core.

## Gradle integration

- Give lint tasks a real report output (`text`, `json`, or `xml`) and declare it
  as `@OutputFile`.
- Mark report-producing lint tasks `@CacheableTask` when output is deterministic.
- Keep source directories, config, plugin version, and analyzer version as
  declared inputs.
- Use Gradle's `InputChanges` API to pass changed source files to the CLI while
  preserving a complete report. If merging partial reports is too complex, use
  the tool cache internally and leave Gradle's task up-to-date check in charge.
- Do not put `threads` in the cache key if output is deterministic across
  thread counts.
- `groovylintFix*` tasks are never cacheable and never part of `check`.

## Performance gates

Measure at least:

| Scenario | Metric |
|---|---|
| Cold full scan | wall time, files/s, peak memory |
| Warm no-change scan | wall time, cache hit ratio |
| One-file change | wall time, re-analyzed files |
| 100-file change | wall time, re-analyzed files |
| Ruleset change | invalidation scope |
| Parser/analyzer version change | full invalidation |
| 10k+ file project | memory and cache size |

Targets, to be refined with real data:

- no unexplained regression greater than 10% per milestone;
- warm no-change should be much cheaper than cold full analysis;
- one-file change should not re-lint the whole project once caching is enabled;
- architecture project rules should remain usable on a 10k-class project;
- cache lookups must not dominate the no-change path.

CI should use relative thresholds or compare against a checked-in baseline, not
absolute wall-clock numbers that vary by runner.

## Correctness tests

- Change a source file: only affected results change.
- Change one rule property: only that rule is invalidated.
- Change the ruleset: all affected rules are invalidated.
- Change the analyzer or parser version: cache is invalidated.
- Delete and rename files: graph edges and findings are updated.
- Corrupt a cache entry: analysis recomputes without failing.
- Disable the cache: results are identical.
- Run concurrently: no partial/corrupt entries.
- Fix mode: cached diagnostics never cause a fix to be applied to the wrong
  source span.

## What not to do

- Do not cache by mtime alone.
- Do not cache findings without the full rule/config/analyzer key.
- Do not cache across a plugin API or parser-schema change.
- Do not add remote/distributed caching before local correctness is proven.
- Do not serialize the AST/CST before profiling shows parse cost is the
  bottleneck.
- Do not make fix tasks cacheable or incremental in a way that can skip a needed
  write.
