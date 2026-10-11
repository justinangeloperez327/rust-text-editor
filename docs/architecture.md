# Architecture

## Product

Windows-first native Rust IDE with a VS Code-style workspace shell.

## Dependency direction

- `apps/desktop` owns windowing, GPUI integration, UI composition, and lifecycle.
- `ide-core` owns UI-independent application vocabulary and services.
- `ide-document` owns document versions, document state, and eventually the text-buffer interface.
- Editor-domain crates may not depend on GPUI or Windows APIs.

## Planned boundaries

1. Application and command orchestration
2. Document/text buffer, view state, selections, undo history
3. Syntax parser and language-server clients
4. Workspace filesystem, project search, Git, terminal, debugger
5. GPUI shell, docking, rendering, input and accessibility
6. Platform adapters for Windows initially

## Engineering requirements

- One authoritative buffer per open document.
- Document-version-aware asynchronous responses.
- No blocking I/O on the UI thread.
- Graceful subprocess failures.
- Safe file writes and external-modification detection.
- Workspace trust before executing repository-supplied code.
- Profile editor working set separately from language-server processes.

## GUI compatibility gate

Group 1 must pass Windows debug and release builds plus interactive checks
for window creation, resize, keyboard input, DirectWrite text, high DPI, and exit.
If GPUI cannot pass, choose a native alternative without changing editor-domain crates.
