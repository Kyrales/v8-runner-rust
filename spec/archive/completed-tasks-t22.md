# Completed Task T22

## T22: Implement universal extension preparation and client MCP tool extension

Status: completed on `2026-05-02`.

Source ADR: [ADR-0022](../decisions/0022-universalnyy-mehanizm-podgotovki-rasshireniy-i-client-mcp-extension.md).

Implemented scope:

- Added `tools.client_mcp.extension` with mutually exclusive `source` and `.cfe` `artifact`
  inputs.
- Kept tool extensions out of project `source-set` ordering and `--source-set` selection.
- Added shared internal tool extension preparation for `build`.
- Made `init` import EDT source tool extension projects into the EDT workspace.
- Kept `launch mcp` and `launch mcp va` from installing or updating the extension, with a
  `v8-runner build` hint when the extension is configured.

Verification:

- `cargo test --locked config::`
- `cargo test --locked client_mcp`
