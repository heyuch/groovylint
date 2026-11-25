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
