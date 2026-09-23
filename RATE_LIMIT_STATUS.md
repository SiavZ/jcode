# Rate Limit Auto-Retry Implementation Status

## Current Status

After investigating the jcode codebase, here's what I found:

### ✅ Already Implemented

1. **Anthropic Runtime** - Full retry_after support:
   - Parses HTTP `Retry-After` headers
   - Wraps errors with retry information
   - Location: `crates/jcode-provider-anthropic-runtime/src/lib.rs:2132-2137`

2. **OpenRouter/OpenCode Runtime** - Partial support:
   - ✅ Parses HTTP `Retry-After` headers in SSE stream
   - ✅ Wraps errors with retry information
   - Location: `crates/jcode-provider-openrouter-runtime/src/openrouter_sse_stream.rs:225-239`
   - **This includes GLM 5.2/5.3 models used via OpenCode**

3. **TUI Rate Limit Handler** - Full support:
   - Detects rate limit errors (429, "rate limit", etc.)
   - Parses retry times from error messages and headers
   - Schedules automatic retry at the specified time
   - Shows status: "Rate limited; queued retry" → "✓ Rate limit reset. Retrying..."
   - Location: `crates/jcode-tui/src/tui/app/`

### ⚠️ Potential Gap

The openrouter runtime:
- ✅ Extracts `Retry-After` from **HTTP headers**
- ❌ Might not parse retry time from **error message body**

For example, if OpenCode/GLM returns:
```json
{
  "error": {
    "message": "Rate limit exceeded. Retry after 30 seconds.",
    "type": "rate_limit_error"
  }
}
```

The HTTP header-based parsing would miss this unless the provider also sends a `Retry-After` header.

## The Fix

I need to enhance the openrouter error handling to also parse the error message body for retry information, similar to how the TUI does it.

### What needs to change:

1. **In `openrouter_sse_stream.rs`** (line 223-239):
   - Currently: Only checks headers
   - Should: Also parse the error body for retry times

2. **Use existing infrastructure**:
   - `jcode_provider_core::retry_after::retry_after()` - for headers ✅
   - Need to add message body parsing using the same logic as `parse_rate_limit_error()`

### Implementation Plan

1. Extract the rate limit parsing logic from TUI into provider-core
2. Update openrouter_sse_stream.rs to parse both headers AND message body
3. Ensure the error wrapper includes the retry_after information

This will ensure GLM 5.2/5.3 and other OpenCode models automatically retry after rate limits.

## Question

Before I implement this, can you share:
1. An example of the actual error message you see when GLM/OpenCode hits a rate limit?
2. Does it include a `Retry-After` HTTP header, or is the retry time only in the error message body?

This will help me implement the most accurate fix.
