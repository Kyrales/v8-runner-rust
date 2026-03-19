# v8-test-runner

Rust CLI for local 1C development workflows.

## Build

Current `build` support is limited to `builder=DESIGNER` and `format=DESIGNER`.

- `v8-test-runner build` runs change detection and loads only affected `source-set` entries.
- `v8-test-runner build --full-rebuild` bypasses change detection and forces full load for every Designer `source-set`.
- Execution order is always the main `CONFIGURATION` first, then extensions in config order.
- Build is intentionally non-atomic across `source-set`: if a later step fails, earlier successful steps remain applied.

Optional YAML settings:

```yaml
build:
  partialLoadThreshold: 20
```

- `partialLoadThreshold` controls when partial load falls back to full load.
- `Configuration.xml` changes and deletions always force a full load.
