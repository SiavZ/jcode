# Pull Request: Add Ctrl+C input clear before quit

## Summary
Adds two-stage Ctrl+C behavior: first press clears the input box, second press proceeds with quit confirmation. This provides a convenient keyboard shortcut to clear input without losing the safety of double-Ctrl+C to quit.

## Motivation
Users often need to clear their input to start over. Currently, this requires:
- Selecting all text and deleting
- Using Ctrl+U (not universally known)
- Backspacing through long input

This change makes Ctrl+C more useful when input is present while maintaining the existing quit safety mechanism.

## Behavior

### Before
1. Ctrl+C → "Press Ctrl+C again to quit"
2. Ctrl+C (within 2s) → Quit

### After
1. **Ctrl+C with non-empty input** → Clears input, shows "Input cleared. Press Ctrl+C again to quit"
2. **Ctrl+C with empty input** → "Press Ctrl+C again to quit"
3. **Ctrl+C (within 2s)** → Quit

## Implementation

**File:** `crates/jcode-tui/src/tui/app/input.rs`

Added a check before `handle_quit_request()`:
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

When the agent is processing, Ctrl+C still interrupts as before (unchanged).

## Benefits
- Quick keyboard shortcut to clear input
- Clears both text and attached images
- Maintains existing quit safety (double Ctrl+C within 2s timeout)
- No breaking changes to existing behavior
- Familiar pattern (many CLIs clear on first Ctrl+C)

## Testing
- [x] Tested with text input
- [x] Tested with attached images
- [x] Verified quit behavior still works
- [x] Verified interrupt behavior during processing unchanged
- [x] Build passes

## Edge Cases Handled
- Empty input: skips clear, goes directly to quit confirmation
- Processing state: continues to interrupt (no change)
- Images attached: cleared along with text
- Cursor position: reset to 0 after clear
