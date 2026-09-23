# Implementing Rate Limit Auto-Retry for OpenRouter/OpenCode

## Problem

OpenRouter runtime (which handles OpenCode/GLM models) currently:
- ✅ Parses `Retry-After` from HTTP headers
- ❌ Doesn't parse retry times from error message bodies

If a provider returns retry information only in the error JSON (not in headers), it won't auto-retry.

## Solution

Add error body parsing to complement the existing header parsing.

## Implementation

### Step 1: Move rate limit parsing to provider-core

The TUI already has excellent rate limit parsing in `helpers_rate_limit_parse.rs`. We should:
1. Move this logic to `jcode-provider-core`
2. Make it available to all providers
3. Keep the TUI version as a thin wrapper

### Step 2: Update openrouter_sse_stream.rs

Current code (line 223-239):
```rust
if !response.status().is_success() {
    let status = response.status();
    let retry_after = jcode_provider_core::retry_after::retry_after(response.headers());
    let body = jcode_base::util::http_error_body(response, "HTTP error").await;
    // ... error formatting ...
    return Err(jcode_provider_core::retry_after::error_with_retry_after(
        error_message,
        retry_after,
    ));
}
```

Enhanced code:
```rust
if !response.status().is_success() {
    let status = response.status();
    let body = jcode_base::util::http_error_body(response, "HTTP error").await;
    
    // Try header first
    let retry_after = jcode_provider_core::retry_after::retry_after(response.headers())
        // Fall back to parsing the error message body
        .or_else(|| jcode_provider_core::retry_after::parse_retry_from_message(&body));
    
    // ... error formatting ...
    return Err(jcode_provider_core::retry_after::error_with_retry_after(
        error_message,
        retry_after,
    ));
}
```

### Step 3: Add parse_retry_from_message to provider-core

```rust
// In jcode-provider-core/src/retry_after.rs

/// Parse retry time from error message body.
/// Looks for patterns like:
/// - "retry after 30 seconds"
/// - "rate limit exceeded. Resets in 2h 5m"
/// - "session limit reached. Try again in 300 seconds"
pub fn parse_retry_from_message(error: &str) -> Option<RetryAfter> {
    parse_retry_duration_from_message(error).map(RetryAfter::new)
}

fn parse_retry_duration_from_message(error: &str) -> Option<Duration> {
    let error_lower = error.to_lowercase();

    // Only process if it looks like a rate/session limit
    if !error_lower.contains("rate limit")
        && !error_lower.contains("rate_limit")
        && !error_lower.contains("session limit")
        && !error_lower.contains("429")
        && !error_lower.contains("too many requests")
    {
        return None;
    }

    // Look for "retry after X seconds"
    if let Some(idx) = error_lower.find("retry") {
        let after = &error_lower[idx..];
        for word in after.split_whitespace() {
            if let Ok(secs) = word
                .trim_matches(|c: char| !c.is_ascii_digit())
                .parse::<u64>()
                && secs > 0
                && secs < 86400
            {
                return Some(Duration::from_secs(secs.min(MAX_RETRY_AFTER.as_secs())));
            }
        }
    }

    // Look for "resets in Xh Ym" format
    if let Some(idx) = error_lower.find("reset") {
        let after = &error_lower[idx..];
        let mut total = Duration::ZERO;
        let mut saw_unit = false;
        
        for word in after.split_whitespace().take(8) {
            let digits: String = word.chars().take_while(|c| c.is_ascii_digit()).collect();
            let rest = &word[digits.len()..];
            
            if digits.is_empty() {
                continue;
            }
            
            let value: u64 = match digits.parse() {
                Ok(v) => v,
                Err(_) => continue,
            };
            
            let secs = match rest.trim_matches(|c: char| !c.is_ascii_alphabetic()) {
                "d" => Some(value * 86400),
                "h" => Some(value * 3600),
                "m" | "min" => Some(value * 60),
                "s" | "sec" => Some(value),
                _ => None,
            };
            
            if let Some(secs) = secs {
                total += Duration::from_secs(secs);
                saw_unit = true;
            }
        }
        
        if saw_unit && total > Duration::ZERO && total < MAX_RETRY_AFTER {
            return Some(total);
        }
    }

    None
}
```

## Testing

Add tests in `retry_after.rs`:

```rust
#[test]
fn parses_retry_from_error_message() {
    assert_eq!(
        parse_retry_duration_from_message("Rate limit exceeded. Retry after 30 seconds."),
        Some(Duration::from_secs(30))
    );
}

#[test]
fn parses_session_limit_message() {
    assert_eq!(
        parse_retry_duration_from_message("Session limit reached. Try again in 300 seconds"),
        Some(Duration::from_secs(300))
    );
}

#[test]
fn ignores_non_limit_errors() {
    assert_eq!(
        parse_retry_duration_from_message("Connection timeout"),
        None
    );
}
```

## Files to Modify

1. `crates/jcode-provider-core/src/retry_after.rs` - Add message parsing
2. `crates/jcode-provider-openrouter-runtime/src/openrouter_sse_stream.rs` - Use message parsing
3. Optional: Apply same fix to other error handlers in openrouter

## Benefits

- Works for providers that send retry time in headers (current)
- Works for providers that send retry time in message body (new)
- Works for providers that send both (uses header, faster)
- Consistent behavior across all OpenAI-compatible providers
- GLM 5.2/5.3 and all OpenCode models will auto-retry correctly

Would you like me to implement this now?
