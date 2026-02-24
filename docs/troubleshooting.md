# Troubleshooting

## macOS IMK log noise

You may occasionally see this line in macOS terminal/app logs:

`error messaging the mach port for IMKCFRunLoopWakeUpReliable`

This message is emitted by macOS Input Method Kit (IMK), not by `simple-term`.

### Impact

- Usually non-fatal and harmless.
- It does not indicate terminal data loss or PTY failure by itself.
- If the terminal UI is still responsive, this log can be treated as noise.

### What to do

1. Verify terminal behavior first (input, redraw, prompt updates, scroll).
2. If behavior is normal, no code change is required.
3. If behavior is degraded, capture reproducible steps and collect:
   - app logs around the timestamp,
   - macOS version,
   - active keyboard/input method,
   - whether issue reproduces with default input source (U.S. keyboard).

### Related note

For `Ctrl+C` redraw responsiveness, rely on terminal perf logs and regression tests under:

- `/Users/mt/Github/zed-terminal/crates/simple-term/tests/terminal_pty_integration.rs`
- `/Users/mt/Github/zed-terminal/apps/simple-term/src/terminal_view.rs`
