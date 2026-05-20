---
name: qmd-search
description: Use when searching local Markdown knowledge bases, project docs, notes, or repository documentation with the qmd BM25 full-text search CLI; index or update collections, search them, and retrieve matching source documents.
---

# QMD Search

Use this skill when the user asks to search, inspect, or answer questions from local Markdown files. QMD is a lightweight BM25-only search engine; it does not perform semantic/vector search.

## Workflow

1. Choose the Markdown root to index.
   - For a project, use the repository or docs directory requested by the user.
   - Keep the index local to the project when possible, such as `.qmd/index.sqlite`.
2. Register or update the collection.
   - Prefer `scripts/qmd_search.sh` for repeatable collection setup and search.
   - Use a stable collection name, for example the repo or docs directory name.
3. Search with focused keyword queries.
   - BM25 rewards lexical overlap, so include exact terms, abbreviations, file names, function names, or domain phrases likely to appear in the Markdown.
   - Run multiple searches when the first query is too broad or too narrow.
4. Retrieve promising documents with `qmd get collection/path.md`.
5. Answer from the retrieved source text. Mention when results are sparse or no matching documents are found.

## Script

Use the bundled script from the skill directory:

```sh
bash skills/qmd-search/scripts/qmd_search.sh \
  --root ./docs \
  --collection docs \
  --query "authentication token" \
  --limit 5
```

Defaults:

- `--index .qmd/index.sqlite`
- `--pattern **/*.md`
- `--limit 10`

The script first tries an installed `qmd` binary. If none is found and it can locate this repository root, it falls back to `cargo run -p qmd-cli --`.

## Direct Commands

When not using the script:

```sh
qmd --index .qmd/index.sqlite collection add ./docs --name docs --pattern "**/*.md"
qmd --index .qmd/index.sqlite update --collection docs
qmd --index .qmd/index.sqlite search "authentication token" -n 5 --json
qmd --index .qmd/index.sqlite get docs/path/to/file.md
```

If using Cargo from this repository, replace `qmd` with:

```sh
cargo run -p qmd-cli -- --index .qmd/index.sqlite
```

