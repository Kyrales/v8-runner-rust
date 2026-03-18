# Agent Rules for v8-test-runner-rust

## After each implementation stage:

1. **Review** — run `/rust-expert-best-practices-code-review` skill on changed code before committing
2. **Compile check** — each stage must produce compilable code (`cargo check` must pass)
3. **Mark progress** — update task status in the task list (in_progress → completed)
4. **Update docs** — update `spec/IMPLEMENTATION_TODO.md` (mark completed items `[x]`) and add/update doc comments (`///`) on all public types, functions, and modules introduced in the stage
5. **Update architecture** — if new modules or significant components are added, update `ARCHITECTURE.md` to reflect the current structure
6. **Commit** — create a git commit with a clear message describing the stage

## Stage definition

A stage is complete when:
- All tasks for that epic/group are marked completed
- `cargo check` passes with no errors
- Rust best practices review has been applied and issues fixed
- `spec/IMPLEMENTATION_TODO.md` is updated: completed items marked with `[x]`
- Public types and functions have `///` doc comments
- `ARCHITECTURE.md` reflects any new modules or components

## Commit message format

```
feat(scope): short description

- bullet points of what was done
```
