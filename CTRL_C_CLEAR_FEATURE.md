# Ctrl+C Input Clear Feature

## Summary

Added a two-stage Ctrl+C behavior to jcode that clears the input box before triggering the quit confirmation.

## How It Works

### Before
- Ctrl+C → "Press Ctrl+C again to quit"
- Ctrl+C (again within 2 seconds) → Quit jcode

### After
1. **First Ctrl+C (input has content):**
   - Clears the input box
   - Clears any pending images
   - Resets cursor position
   - Shows: "Input cleared. Press Ctrl+C again to quit"

2. **Second Ctrl+C (input already empty):**
   - Shows: "Press Ctrl+C again to quit"

3. **Third Ctrl+C (or second within 2s timeout):**
   - Quits jcode

## Benefits

- Provides a quick keyboard shortcut to clear input without:
  - Selecting all text and deleting
  - Using Ctrl+U (which may not be familiar to all users)
  - Backspacing through long input
- Maintains the existing safety mechanism (double Ctrl+C to quit)
- Works with both text and image attachments

## Implementation

**File:** `crates/jcode-tui/src/tui/app/input.rs`

Added a condition before calling `handle_quit_request()`:
```rust
} else if !app.input.is_empty() {
    // First Ctrl+C: clear the input box
    app.input.clear();
    app.pending_images.clear();
    app.cursor_pos = 0;
    app.set_status_notice("Input cleared. Press Ctrl+C again to quit");
} else {
    // Second Ctrl+C (input already empty): proceed with quit
    app.handle_quit_request();
}
```

## Behavior When Processing

When the agent is actively processing (streaming a response), Ctrl+C still interrupts the processing as before. This new behavior only applies when idle.

## Commit

- **Commit:** 92765e755
- **Message:** "feat(tui): clear input on first Ctrl+C, quit on second"

## Testing

Build and test:
```bash
cd /Users/siavash/Projects/jcode
cargo build --release -p jcode --bin jcode
./target/release/jcode
```

Try it:
1. Type some text in jcode
2. Press Ctrl+C → Input clears
3. Press Ctrl+C → Quit confirmation
4. Press Ctrl+C → Exits

## Version

Available in: jcode v0.86.2-dev (local build)
