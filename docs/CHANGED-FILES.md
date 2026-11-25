# Changed files only

A branch should be judged by what the branch changed. Checking 24,000 files to
find the three a pull request touched costs time and buries the findings a reviewer
actually needs to act on, and it reports debt the author did not create.

Two ways to restrict the analysis, both shared by `check` and `nullcheck`.

## 1. A list the build already computed

```bash
groovylint check --changed-files build/changed-groovy-files.txt .
```

The file holds one path per line; blank lines and `#` comments are ignored, and
relative paths resolve against the working directory. `-` reads stdin, which keeps
the caller free of temporary files:

```bash
git diff --name-only --diff-filter=ACMR origin/develop...HEAD | groovylint check --changed-files - .
```

This is the shape a Gradle or Maven build wants: one script computes the set once,
every static-analysis tool consumes the same list, and the totals stay consistent
between tools. Paths that are not Groovy sources, or that no longer exist, are
simply not selected — the list may contain resources or deleted files.

## 2. `git` directly

| Option | `git` equivalent | Use it for |
|---|---|---|
| `--changed-baseline <ref>` | `git diff --diff-filter=ACMR <ref>...HEAD` **+** `git diff HEAD` **+** `git ls-files --others --exclude-standard` | "everything this branch changed" — the normal CI mode (`origin/develop`) |
| `--changed-since <ref>` | `git diff --diff-filter=ACMR <ref>` **+** the same working-tree sources | after a merge, `--changed-since HEAD^1` looks only at what the merge brought in |
| `--changed-commits <a,b>` | per commit: `git show --first-parent --root --pretty=format:` `--name-only --diff-filter=ACMR` | "what did these commits change", ignoring the working tree |

The three modes are deliberately distinct, because they answer different questions
and mixing them up is the classic mistake:

- the **baseline** mode is a *three-point* diff (`<ref>...HEAD`). A two-point diff
  would count commits that landed on the baseline branch after this branch forked
  as "our changes";
- **`--changed-since`** is a *two-point* diff, which is what you want when the
  comparison point is a commit you already merged;
- **`--changed-commits`** never looks at the working tree, so uncommitted edits and
  new files do not leak into a "what did this commit do" check.

Deleted files are dropped (`--diff-filter=ACMR`), renamed files count as the new
path, and untracked files are included in the first two modes — a new file that is
not committed yet is still a change you want checked.

## Narrowing the scope further

`--exclude <SUBSTRING>` drops every path that contains the given text, and it can
be repeated. It exists to line the local run up with whatever CI actually checks:

```bash
# CI only lints main sources
groovylint check --config codenarc.xml --exclude /src/test/ --changed-baseline origin/develop .

# CI lints test sources too, but not the test classes themselves
groovylint check --config codenarc.xml --exclude Test.groovy --changed-baseline origin/develop .
```

The patterns are substrings, not globs, because the intent is always "somewhere in
this path" and a substring is one rule instead of a pattern language. Excluded
paths are counted out before the files are parsed, so a run that skips the test
tree costs less, and the summary line says what was skipped:

```text
96 warning(s) in 300 file(s) (300 file(s) checked, changed since origin/develop...HEAD
  + working tree (391 path(s) in the set), excluding Test.groovy)
```

## Cost

A changed-only run must be cheaper than a full run, otherwise it is only a
reporting filter. It is, but the first implementation was not, and the reason is
worth keeping in mind when extending this code:

| Cost | Full run (24k files) | Walk-then-filter (first attempt) |
|---|---:|---:|
| enumerate the tree | paid | paid anyway |
| resolve every path (symlinks) to match the changed set | — | paid |
| `git ls-files --others` (whole tree, honours .gitignore) | — | paid |
| parse | 24k files | 1 file |

So the run that checked one file spent *more* time than the run that checked all of
them: it paid the enumeration and the `git` walk and saved only the parsing. Two
changes fixed it:

1. **Resolve the changed set instead of walking.** A changed set is small, so each
   entry is canonicalized once and checked against the requested paths; the tree is
   never enumerated. This alone took 1.1 s to 0.24 s.
2. **Scope `git` to the analyzed paths.** The paths on the command line are passed
   as pathspecs (`git diff … -- <paths>`), so `git` walks the requested subtree
   instead of the repository. A module-scoped run drops to ~60 ms.

Measured on a 24,596-file project with a warm cache: full 700–950 ms, changed over
the whole repository ~245 ms, changed within one module ~60 ms, list of one file
~6 ms. The remaining cost of the whole-repository mode is `git ls-files --others`
(~190 ms): finding untracked files requires walking the working tree. That is the
price of "a new file I have not added yet is part of my change"; a module scope or a
list file avoids it.

## Behaviour

- The filter intersects the changed set with the files the arguments select, so
  `--changed-baseline origin/develop gi-policy/gi-policy-domain` checks the changed
  files of that module only.
- **Zero changed files is a success**, with a message on stderr
  (`no Groovy source files to check (0 of 24596 file(s) checked, changed since
  origin/develop...HEAD + working tree); nothing to do`). A green build with
  nothing checked must never be mistaken for a green build that checked everything,
  which is why the summary line always carries the counts.
- **An unknown ref fails** with exit code 2 instead of silently checking nothing.
  A typo in a ref must not turn the gate off.
- Refs are validated before they reach `git`: letters, digits and `. _ / ~ ^ @ { } : -`,
  not starting with `-`, so `--changed-since=--output=/tmp/x` is rejected rather than
  passed through as an option.
- The JSON output is unchanged; the counts are only added to the text summary.

## When the environment has no git

Nothing in the analysis path needs git: `--changed-files` reads a list and matches
it against the files the arguments select. So the answer is to move *who computes
the set* to wherever git does exist, and keep the analyzer git-free:

| Environment | What computes the set | What the analyzer gets |
|---|---|---|
| Developer machine | the build (`changed-files.gradle`) or `git diff` | `--changed-files <file>` |
| CI with a git checkout | the CI step, before the container starts | `--changed-files -` (piped) |
| Container image without git | the CI runner or a previous job | list file mounted into the container, or stdin |
| Source archive (no `.git`) | the SCM API that produced the archive | list file |
| Maven/Gradle build on a fresh clone | the build script, if git is present; otherwise a file the CI wrote | `--changed-files <file>` |

Two shapes cover all of them.

Piped, when the runner has git but the analyzer runs in a container:

```bash
git diff --name-only --diff-filter=ACMR origin/develop...HEAD \
  | docker run --rm -i -v "$PWD:/w" -w /w groovylint check --changed-files - .
```

A file written by an earlier step, when nothing has a repository:

```bash
# step 1 (has git or an SCM API)
python3 ci/changed_files.py > build/changed-groovy-files.txt
# step 2 (no git)
groovylint check --changed-files build/changed-groovy-files.txt .
```

On the Gradle side the same thing is one property; the plugin does not run git
either, so it never needs it:

```groovy
groovylint {
    changedFilesList.set(layout.projectDirectory.file("build/changed-groovy-files.txt"))
}
```

`changedFilesList` is passed to the analyzer unchanged and implies changed-only
mode, so a build without git can still be a changed-only build.

The failure mode is deliberate: asking for `--changed-baseline`,
`--changed-since` or `--changed-commits` in an environment without git exits with
code 2 and this message, rather than checking nothing and reporting success:

```
git is not available in this environment, so the changed set cannot be computed
here; pass it explicitly with --changed-files <FILE|-> (…)
```

Related pitfall, same family: a **shallow clone** has git but not the history.
`--changed-baseline origin/develop` then fails to resolve the ref (exit 2, "run
`git fetch` first; a shallow clone needs `git fetch --deepen`"). CI checkouts
default to `fetch-depth: 1`, so a changed-only gate needs `fetch-depth: 0` or an
explicit `git fetch --deepen`.

## Not covered

- **Architecture rules are project-wide** and ignore the filter: a rule about
  module dependencies cannot be evaluated from three files. Run them once per build,
  not per changed file.
- **Line-level filtering** is not implemented: a file that changed is checked
  completely. Filtering to the changed *lines* would hide existing problems in the
  same file and make the result depend on diff hunks — a different product decision,
  not a performance trick.
- **Baselines** (`docs/local/`) and changed-files filtering solve different
  problems: the filter answers "who touched it now", a baseline answers "we accept
  these existing violations". Use both for adoption: filter in the pre-push hook,
  baseline in CI.
