# v8-runner

`v8-runner` is a Rust CLI and MCP server for local 1C development workflows. It gives a single
entrypoint for building source sets into an infobase, validating changes, running tests, dumping
state back to files, materializing release artifacts, converting between `DESIGNER` and `EDT`,
launching 1C tools, and exposing a narrower MCP tool surface for agents.

## Overview

- Main user loop: `build -> syntax/test -> diagnose`.
- Source formats: `DESIGNER` and `EDT`.
- Builder backends: `DESIGNER` and `IBCMD` where the command contract allows it.
- Machine-readable CLI output: `--json-message`.
- MCP `structured_content` uses the same command envelope core as CLI JSON.

## Quick Start

Build the binary:

```bash
cargo build --release
```

Create a starter config in the current repository:

```bash
./target/release/v8-runner config init
```

Run the usual local loop:

```bash
./target/release/v8-runner init
./target/release/v8-runner build
./target/release/v8-runner syntax designer-modules --server
./target/release/v8-runner test yaxunit all
```

Start MCP transport when needed:

```bash
./target/release/v8-runner mcp serve stdio
```

If `config init` is not enough for your repository layout, use
[docs/CONFIGURATION.md](docs/CONFIGURATION.md) to author `v8project.yaml` manually.

## Support Snapshot

| Area | Current contract |
| --- | --- |
| Project setup | `config init`, `init`, `extensions`, `build` |
| Verification | `test`, `syntax` |
| File materialization | `dump`, `convert`, `load`, `make` / `artifacts` |
| Direct launch | `launch <designer|thin|thick|ordinary>`, `launch mcp [va] --mode=<thin|thick|ordinary>` |
| MCP | `mcp serve stdio|http`, 8 published tools |

## Documentation Map

- [docs/CAPABILITIES.md](docs/CAPABILITIES.md): full command catalog, support matrix, MCP tool list, and current limitations.
- [docs/CONFIGURATION.md](docs/CONFIGURATION.md): `v8project.yaml` contract, supported keys, aliases, and config-specific limits.
- [docs/DEEP_DIVE.md](docs/DEEP_DIVE.md): execution semantics, runtime model, lock/publication behavior, and operational nuances.
- [docs/README.md](docs/README.md): documentation stack and source-of-truth order.
- [ARCHITECTURE.md](ARCHITECTURE.md): contributor-facing module and boundary map.
- [spec/README.md](spec/README.md): active internal planning, ADR, architecture-rule, and acceptance entrypoints.
- [references/1c/README.md](references/1c/README.md): raw external 1C reference corpus, not project source of truth.
