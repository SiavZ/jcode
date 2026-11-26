//! Successful live quota probes, kept separate from cached display snapshots.
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::Instant;

static AVAILABLE: LazyLock<Mutex<HashMap<(String, String), i64>>> = LazyLock::new(Mutex::default);

pub(super) fn record_available(provider: &str, label: &str, probe_started: Instant) {
    if let Some(kind) = jcode_provider_core::AccountProviderKind::from_key(provider) {
        crate::provider::models::clear_quota_unavailability_observed_before(
            provider,
            label,
            probe_started,
        );
        crate::provider::account_failover::clear_account_exhausted_before(
            kind,
            label,
            probe_started,
        );
        if crate::provider::account_failover::account_exhausted(kind, label).is_some()
            || crate::provider::models::provider_unavailability_detail_for_account_label(
                provider,
                Some(label),
            )
            .is_some()
        {
            return;
        }
    }
    let at = chrono::Utc::now().timestamp_millis()
        - i64::try_from(probe_started.elapsed().as_millis()).unwrap_or(i64::MAX);
    let mut available = AVAILABLE.lock().unwrap_or_else(|p| p.into_inner());
    let entry = available
        .entry((provider.into(), label.into()))
        .or_default();
    *entry = (*entry).max(at);
}

/// A probe started at this time positively confirmed headroom for this account.
/// Reusing a cached report never advances this timestamp.
pub fn confirmed_available_at(provider: &str, label: &str) -> Option<i64> {
    AVAILABLE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .get(&(provider.into(), label.into()))
        .copied()
}

pub fn confirmed_available_accounts() -> Vec<jcode_protocol::AccountUsageRecovery> {
    AVAILABLE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .iter()
        .map(
            |((provider, label), at)| jcode_protocol::AccountUsageRecovery {
                provider: provider.clone(),
                account_label: label.clone(),
                checked_at_unix_ms: *at,
            },
        )
        .collect()
}

pub async fn refresh_usage_limit_recovery(provider: &str, label: &str) {
    if provider == "claude-code" {
        super::claude_code::refresh_recovery(label).await;
    } else {
        let _ = tokio::time::timeout(
            std::time::Duration::from_secs(20),
            super::fetch_saved_account_usage(provider, label),
        )
        .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_is_account_scoped_and_uses_probe_start_not_completion() {
        let started = Instant::now() - std::time::Duration::from_secs(60);
        let before = chrono::Utc::now().timestamp_millis();
        record_available("openai", "recovery-test", started);
        let at = confirmed_available_at("openai", "recovery-test").unwrap();
        assert!(at <= before - 59_000);
        assert!(confirmed_available_at("claude", "recovery-test").is_none());
        assert!(confirmed_available_at("openai", "other-recovery-test").is_none());
        record_available(
            "openai",
            "recovery-test",
            started - std::time::Duration::from_secs(60),
        );
        assert_eq!(confirmed_available_at("openai", "recovery-test"), Some(at));
    }

    #[test]
    fn fresh_usage_clears_exhaustion_but_cannot_erase_a_newer_denial() {
        use jcode_provider_core::AccountProviderKind;
        let label = "recovery-ledger-test";
        let kind = AccountProviderKind::ClaudeCode;
        let old_probe = Instant::now() - std::time::Duration::from_secs(60);
        crate::provider::account_failover::record_account_exhausted(kind, label, None);
        record_available("claude-code", label, old_probe);
        assert!(crate::provider::account_failover::account_exhausted(kind, label).is_some());
        assert!(confirmed_available_at("claude-code", label).is_none());
        record_available("claude-code", label, Instant::now());
        assert!(crate::provider::account_failover::account_exhausted(kind, label).is_none());
        assert!(confirmed_available_at("claude-code", label).is_some());
    }
}
