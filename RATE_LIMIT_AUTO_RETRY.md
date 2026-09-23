# Rate Limit Auto-Retry Feature in jcode

## Summary

**Good news!** The feature you described is **already implemented** in jcode. When a model provider returns a rate limit error with a retry time, jcode automatically holds the request and retries at the specified time.

## How It Works

### 1. Rate Limit Detection

When a provider returns an error, jcode checks if it's a rate limit error by looking for:
- HTTP 429 status codes
- Error messages containing "rate limit", "rate_limit", "too many requests", or "hit your limit"

**Files:**
- `crates/jcode-tui/src/tui/app/helpers_rate_limit_parse.rs` - Parses error messages
- `crates/jcode-provider-core/src/retry_after.rs` - Parses HTTP `Retry-After` headers

### 2. Retry Time Extraction

jcode extracts the retry time from multiple sources:
- **HTTP `Retry-After` header** (seconds or HTTP date format)
- **Error message patterns**:
  - "retry after 30 seconds"
  - "resets in 2h 5m"
  - "resets in 30d 4h 29m" (parsed but only auto-retries if < 24 hours)
  - Clock times like "resets at 3:45pm"

### 3. Automatic Scheduling

When a rate limit is detected:

```rust
// From server_events.rs:1268-1272
let reset_duration = retry_after_secs
    .map(Duration::from_secs)
    .or_else(|| parse_rate_limit_error(&message));
if let Some(reset_duration) = reset_duration {
    app.rate_limit_reset = Some(Instant::now() + reset_duration);
    // ... stores pending message for retry
}
```

The app:
1. Sets `rate_limit_reset` to the future retry time
2. Stores the failed request in `rate_limit_pending_message`
3. Shows a message: "Rate limited; queued retry"
4. Stops processing and returns to idle

### 4. Automatic Retry

On each tick, jcode checks if the retry time has passed:

```rust
// From local.rs:121-133
if let Some(reset_time) = app.rate_limit_reset
    && Instant::now() >= reset_time
{
    app.rate_limit_reset = None;
    app.push_display_message(DisplayMessage::system("✓ Rate limit reset. Retrying..."));
    app.pending_turn = true;  // Triggers automatic retry
}
```

When the time arrives:
- Clears the rate limit state
- Shows "✓ Rate limit reset. Retrying..."
- Automatically resends the request

## Supported Providers

All providers that use the shared infrastructure support this:
- **Anthropic** (Claude) - via `jcode-provider-anthropic-runtime`
- **OpenAI** (GPT, Codex) - via standard retry infrastructure
- **Claude CLI** - has dedicated `retry_after_secs` field
- **OpenRouter** and other providers using reqwest

## Limits and Safety

1. **Maximum wait time:** 60 seconds (`MAX_RETRY_AFTER`)
   - Longer waits are capped to prevent indefinite stalls
   - Messages > 24 hours are not auto-retried (user must manually retry)

2. **Retry attempts:** Limited by `max_attempts` parameter
   - Prevents infinite retry loops
   - Different limits for different error types

3. **Network awareness:**
   - Detects offline state and waits for network before retrying
   - Shows "Network appears offline - waiting to retry automatically"

## Example Error Messages Handled

✅ `429 Too Many Requests: retry after 30 seconds`
✅ `429 rate limit exceeded. Resets in 2h 5m.`
✅ `Rate limited: The usage limit has been reached. Resets in 30d 4h 29m (2026-08-21 04:31 UTC).`
✅ HTTP header: `Retry-After: 120`
✅ HTTP header: `Retry-After: Wed, 21 Oct 2026 07:28:00 GMT`

❌ `Resets in 30d 4h 29m` - Too long (>24h), marked terminal, no auto-retry

## User Experience

### When Rate Limited

```
[System message appears in chat]
Rate limited; queued retry

[Status bar shows]
Idle
```

### When Retry Time Arrives

```
[System message appears in chat]
✓ Rate limit reset. Retrying...

[Request automatically resends]
```

### With Queued Messages

```
✓ Rate limit reset. Retrying... (+2 queued)
```

## What Happens Behind the Scenes

1. **Request fails** with rate limit error
2. **Parse** retry time from error/headers
3. **Store** the original request payload
4. **Schedule** retry for the specified time
5. **Wait** (app remains responsive, user can still interact)
6. **Retry** automatically when time arrives
7. **Continue** as if nothing happened

## Configuration

No configuration needed - this feature is automatic and always enabled.

## Code Locations

- **Rate limit parsing:** `crates/jcode-tui/src/tui/app/helpers_rate_limit_parse.rs`
- **Retry-After header:** `crates/jcode-provider-core/src/retry_after.rs`
- **Error detection:** `crates/jcode-tui/src/tui/app/remote/server_events.rs:1268`
- **Automatic retry:** `crates/jcode-tui/src/tui/app/local.rs:121`
- **App state:** `crates/jcode-tui/src/tui/app.rs:1578-1580`

## Tests

The feature is well-tested:
- `helpers_rate_limit_parse.rs:100-126` - Message parsing tests
- `retry_after.rs:114-219` - Header parsing tests
- `remote_tests.rs:613,693,891` - Integration tests

---

## Summary

**The feature you requested already exists!** When a provider hits a rate limit and specifies a reset time, jcode:
- ✅ Recognizes the error
- ✅ Extracts the retry time
- ✅ Holds the request
- ✅ Automatically retries when the time arrives
- ✅ Shows clear status messages

No changes needed - it's working as designed!
