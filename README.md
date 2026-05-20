# qmd

[![CI][ci-badge]][ci-url]
[![License][license-badge]][license-url]
[![Rust][rust-badge]][rust-url]

[ci-badge]: https://github.com/qntx/qmd/actions/workflows/rust.yml/badge.svg
[ci-url]: https://github.com/qntx/qmd/actions/workflows/rust.yml
[license-badge]: https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg
[license-url]: LICENSE-MIT
[rust-badge]: https://img.shields.io/badge/rust-edition%202024-orange.svg
[rust-url]: https://doc.rust-lang.org/edition-guide/

**Lightweight local BM25 search engine for Markdown files in Rust.**

qmd indexes Markdown collections into a single SQLite database and searches them
with SQLite FTS5/BM25. It is intentionally dependency-light: no embedding model,
no ONNX runtime, and no vector database.

## Features

- Register one or more Markdown collections.
- Incrementally index changed files by content hash.
- Search with BM25 full-text ranking.
- Retrieve documents by `collection/path.md` or short document id.
- Store optional global and path-scoped context notes for collections.
- Use as a Codex skill backend through [`skills/qmd-search`](skills/qmd-search/).

## Crates

| Crate | | Description |
| --- | --- | --- |
| **[`qmd`](qmd/)** | [![crates.io][qmd-crate]][qmd-crate-url] [![docs.rs][qmd-doc]][qmd-doc-url] | Core library — indexing and BM25 full-text search |
| **[`qmd-cli`](qmd-cli/)** | [![crates.io][cli-crate]][cli-crate-url] | CLI tool — collection management and search |

[qmd-crate]: https://img.shields.io/crates/v/qmd.svg
[qmd-crate-url]: https://crates.io/crates/qmd
[cli-crate]: https://img.shields.io/crates/v/qmd-cli.svg
[cli-crate-url]: https://crates.io/crates/qmd-cli
[qmd-doc]: https://img.shields.io/docsrs/qmd.svg
[qmd-doc-url]: https://docs.rs/qmd

## Quick Start

### Install the CLI

**Shell** (macOS / Linux):

```sh
curl -fsSL https://sh.qntx.fun/qmd | sh
```

**PowerShell** (Windows):

```powershell
irm https://sh.qntx.fun/qmd/ps | iex
```

Or via Cargo:

```bash
cargo install qmd-cli
```

### CLI Usage

```bash
# Add a collection of markdown files
qmd --index .qmd/index.sqlite collection add ./docs --name my-docs --pattern "**/*.md"

# List collections
qmd --index .qmd/index.sqlite collection list

# BM25 full-text search
qmd --index .qmd/index.sqlite search "query expansion" -n 5

# Get a specific document
qmd --index .qmd/index.sqlite get my-docs/README.md

# Re-index all collections
qmd --index .qmd/index.sqlite update
```

### Codex Skill

This repository includes a local Codex skill at
[`skills/qmd-search`](skills/qmd-search/). The skill teaches an agent how to use
qmd as a BM25 search backend for local Markdown knowledge bases.

Example:

```bash
bash skills/qmd-search/scripts/qmd_search.sh \
  --index .qmd/index.sqlite \
  --root ./docs \
  --collection docs \
  --query "authentication token" \
  --limit 5
```

The script registers the collection, updates the index, and prints JSON search
results. It uses an installed `qmd` binary when available, otherwise it falls
back to this repository's `qmd-cli` via Cargo.

## Development

```bash
cargo check --workspace --all-targets
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project shall be dual-licensed as above, without any additional terms or conditions.
