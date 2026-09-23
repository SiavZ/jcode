# Pull Request: Fix clipboard image paste priority

## Summary
Fixes clipboard image paste on macOS (and other platforms) by prioritizing images over text in Smart paste mode, matching Claude Code behavior.

## Problem
When copying an image and pasting with `Cmd+V`, jcode would paste text (like a file path) instead of the actual image. This happened because macOS clipboard often contains both image data and text when an image is copied, and jcode was checking for text first.

## Solution
Changed the order in `read_clipboard_for_paste_with()` to check for images **before** text in Smart paste mode. This ensures that when both are available, the image is pasted (as users expect).

## Changes
- Modified `crates/jcode-tui/src/tui/app/input.rs`:
  - Reordered clipboard checks in `ClipboardPasteKind::Smart` to prioritize images
  - Updated test `smart_paste_prefers_image_when_clipboard_has_both` (renamed from `smart_paste_prefers_normal_text_when_clipboard_has_text`)
  - Added new test `smart_paste_uses_text_when_no_image_is_available`

## Testing
- [x] Updated existing tests to reflect new behavior
- [x] Added test for text-only paste fallback
- [x] Manually tested on macOS with screenshots and copied images
- [x] Verified text-only paste still works when no image is present

## Behavior Changes
**Before:** Smart paste checks text first, then images
- Copy image → Paste → Get text (file path)

**After:** Smart paste checks images first, then text
- Copy image → Paste → Get image ✅
- Copy text → Paste → Get text ✅

## Compatibility
- All platforms (macOS, Linux/Wayland, Linux/X11)
- Backwards compatible
- `ImageOnly` paste mode unchanged (already prioritized images)

## Related
- Feature originally added in v0.66.0 (August 2026)
- Clipboard infrastructure already supports images via osascript, wl-paste, and arboard
