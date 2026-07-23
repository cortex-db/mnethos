//! Data-transfer objects for the Mnethos client backend contract.
//!
//! These types intentionally mirror the structures the CLI client
//! deserializes from the `auth/user` and `auth/usage` endpoints
//! (`crates/forge_app/src/user.rs`). The JSON field names use `camelCase`
//! to stay byte-compatible with what the client expects over the wire.

use derive_setters::Setters;
use serde::{Deserialize, Serialize};

/// Opaque identity of the authenticated principal as returned by `auth/user`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AuthProviderId(String);

impl AuthProviderId {
    /// Creates a new identifier from any string-like value.
    pub fn new(id: impl ToString) -> Self {
        Self(id.to_string())
    }

    /// Consumes the identifier and returns the inner string.
    pub fn into_string(self) -> String {
        self.0
    }

    /// Returns the identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Response body for `GET auth/user`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Setters)]
#[setters(into)]
#[serde(rename_all = "camelCase")]
pub struct User {
    /// Identity of the authenticated principal.
    pub auth_provider_id: AuthProviderId,
}

impl User {
    /// Creates a user payload for the given principal identifier.
    pub fn new(auth_provider_id: impl ToString) -> Self {
        Self { auth_provider_id: AuthProviderId::new(auth_provider_id) }
    }
}

/// Subscription plan associated with an account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    /// Plan tier identifier (e.g. `free`, `pro`, `enterprise`).
    pub r#type: String,
}

impl Plan {
    /// Creates a plan with the given tier identifier.
    pub fn new(r#type: impl ToString) -> Self {
        Self { r#type: r#type.to_string() }
    }
}

/// Request-quota usage information for an account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Setters)]
#[setters(into, strip_option)]
#[serde(rename_all = "camelCase")]
pub struct UsageInfo {
    /// Number of requests consumed in the current window.
    pub current: u32,
    /// Maximum number of requests permitted in the current window.
    pub limit: u32,
    /// Requests still available in the current window.
    pub remaining: u32,
    /// Seconds until the quota window resets, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reset_in: Option<u64>,
}

impl UsageInfo {
    /// Creates usage information from the consumed and total request counts,
    /// deriving `remaining` as `limit.saturating_sub(current)`.
    pub fn new(current: u32, limit: u32) -> Self {
        Self {
            current,
            limit,
            remaining: limit.saturating_sub(current),
            reset_in: None,
        }
    }
}

/// Response body for `GET auth/usage`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Setters)]
#[setters(into)]
#[serde(rename_all = "camelCase")]
pub struct UserUsage {
    /// Subscription plan for the account.
    pub plan: Plan,
    /// Current request-quota usage for the account.
    pub usage: UsageInfo,
}

impl UserUsage {
    /// Creates a usage payload from a plan and usage breakdown.
    pub fn new(plan: Plan, usage: UsageInfo) -> Self {
        Self { plan, usage }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use serde_json::json;

    use super::*;

    #[test]
    fn test_user_serializes_to_client_contract() {
        let fixture = User::new("user-123");
        let actual = serde_json::to_value(&fixture).unwrap();
        let expected = json!({ "authProviderId": "user-123" });
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_user_usage_serializes_to_client_contract() {
        let fixture = UserUsage::new(Plan::new("pro"), UsageInfo::new(10, 100).reset_in(3600u64));
        let actual = serde_json::to_value(&fixture).unwrap();
        let expected = json!({
            "plan": { "type": "pro" },
            "usage": { "current": 10, "limit": 100, "remaining": 90, "resetIn": 3600 }
        });
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_usage_info_omits_reset_in_when_absent() {
        let fixture = UserUsage::new(Plan::new("free"), UsageInfo::new(5, 50));
        let actual = serde_json::to_value(&fixture).unwrap();
        let expected = json!({
            "plan": { "type": "free" },
            "usage": { "current": 5, "limit": 50, "remaining": 45 }
        });
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_usage_info_derives_remaining() {
        let actual = UsageInfo::new(30, 100);
        let expected = UsageInfo { current: 30, limit: 100, remaining: 70, reset_in: None };
        assert_eq!(actual, expected);
    }
}
