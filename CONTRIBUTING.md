# Contributing

Thanks for your interest in `groovylint`.

> **Early implementation:** a parser subset and the first rule slice exist.
> Parser coverage, CodeNarc compatibility, and semantic analysis are still under
> active design. Please open an issue before substantial parser or rule-engine
> changes; see [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

## Prerequisites

- Rustup. The repository pins Rust `1.100.0` via `rust-toolchain.toml`; Rustup
  installs it automatically.
- Git.

## Local workflow

```bash
git clone <repository-url>
cd groovylint

cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Before opening a pull request, run:

```bash
cargo fmt-check
cargo lint
cargo test-all
```

## Code standards

- Format with `rustfmt`; do not hand-format code.
- Treat Clippy warnings as errors. Prefer fixing the code over adding
  `#[allow(...)]`; if an allow is necessary, explain why in a comment.
- Keep public APIs documented.
- Do not add dependencies without discussing their maintenance, license, and
  MSRV impact.
- Add tests with behavior changes. For parser or analysis changes, include a
  minimal regression fixture.
- Keep commits focused and use imperative commit subjects.

## Adding a crate or module

The current workspace layout is provisional. Before adding a new crate:

1. Explain which dependency direction it enables.
2. Check that the responsibility cannot live in an existing crate.
3. Add workspace-level lint inheritance and package metadata.
4. Update `docs/ARCHITECTURE.md`.

## Pull requests

A useful pull request includes:

- the problem and the chosen approach;
- tests that fail before the change and pass after it;
- any compatibility or performance trade-offs;
- documentation updates when public behavior changes.

## License

By contributing, you agree that your contributions are licensed under the MIT
License.
