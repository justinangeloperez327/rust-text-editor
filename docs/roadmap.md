# Development roadmap

| Stage | Groups | Result |
| --- | --- | --- |
| A: Foundation | 1–3 | Windows application, state, VS Code-style shell |
| B: Engine | 4–7 | Reliable editing, virtualized rendering, Tree-sitter |
| C: Workspace | 8–10 | Explorer, search, command palette, settings |
| D: Languages | 11–13 | LSP, completions, diagnostics, formatting |
| E: IDE | 14–17 | Terminal, Git, tasks, DAP debugger |
| F: Release | 18–20 | Profiling, recovery, security, installer |

## Group 1 acceptance

- [ ] Cargo workspace initialized
- [ ] Native app launches on Windows
- [ ] Debug and release builds succeed on Windows CI
- [ ] Mouse/keyboard, text rendering, scaling, resize and exit manually verified
- [ ] Baseline cold start and RAM measured
- [ ] Core crates have no GUI dependencies

Group 1 is not accepted until runtime behavior is verified on Windows.
