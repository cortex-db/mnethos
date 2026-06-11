//! Account lookup backing the authentication endpoints.

use std::collections::HashMap;

use crate::dto::{User, UserUsage};

/// Read-only lookup of account information keyed by bearer API key.
///
/// Implementations resolve the opaque API key presented in the
/// `Authorization: Bearer <key>` header into the user identity and usage
/// payloads served by the `auth/user` and `auth/usage` endpoints.
pub trait UserStore: Send + Sync {
    /// Returns the user identity for the given API key, or `None` when the key
    /// is unknown.
    fn user(&self, api_key: &str) -> Option<User>;

    /// Returns the usage breakdown for the given API key, or `None` when the
    /// key is unknown.
    fn usage(&self, api_key: &str) -> Option<UserUsage>;
}

/// A single account entry pairing identity with usage.
#[derive(Debug, Clone)]
struct Account {
    user: User,
    usage: UserUsage,
}

/// In-memory [`UserStore`] backed by a `HashMap` from API key to account.
///
/// Intended for local development and as a reference backend. Accounts are
/// registered up-front via [`InMemoryUserStore::with_account`].
#[derive(Debug, Clone, Default)]
pub struct InMemoryUserStore {
    accounts: HashMap<String, Account>,
}

impl InMemoryUserStore {
    /// Creates an empty store with no registered accounts.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers an account under the given API key, returning `self` for
    /// builder-style chaining. An existing entry for the key is replaced.
    pub fn with_account(
        mut self,
        api_key: impl Into<String>,
        user: User,
        usage: UserUsage,
    ) -> Self {
        self.accounts
            .insert(api_key.into(), Account { user, usage });
        self
    }

    /// Builds a store pre-seeded with a single demo account.
    ///
    /// The demo account uses the API key `mnethos-demo-key`, principal
    /// `demo-user`, a `pro` plan and a 10/100 request quota.
    pub fn demo() -> Self {
        use crate::dto::{Plan, UsageInfo};

        Self::new().with_account(
            "mnethos-demo-key",
            User::new("demo-user"),
            UserUsage::new(Plan::new("pro"), UsageInfo::new(10, 100)),
        )
    }
}

impl UserStore for InMemoryUserStore {
    fn user(&self, api_key: &str) -> Option<User> {
        self.accounts
            .get(api_key)
            .map(|account| account.user.clone())
    }

    fn usage(&self, api_key: &str) -> Option<UserUsage> {
        self.accounts
            .get(api_key)
            .map(|account| account.usage.clone())
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::dto::{Plan, UsageInfo};

    fn fixture() -> InMemoryUserStore {
        InMemoryUserStore::new().with_account(
            "key-abc",
            User::new("alice"),
            UserUsage::new(Plan::new("free"), UsageInfo::new(1, 10)),
        )
    }

    #[test]
    fn test_known_key_resolves_user() {
        let actual = fixture().user("key-abc");
        let expected = Some(User::new("alice"));
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_known_key_resolves_usage() {
        let actual = fixture().usage("key-abc");
        let expected = Some(UserUsage::new(Plan::new("free"), UsageInfo::new(1, 10)));
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_unknown_key_resolves_none() {
        let fixture = fixture();
        assert_eq!(fixture.user("nope"), None);
        assert_eq!(fixture.usage("nope"), None);
    }

    #[test]
    fn test_demo_store_has_demo_key() {
        let actual = InMemoryUserStore::demo().user("mnethos-demo-key");
        let expected = Some(User::new("demo-user"));
        assert_eq!(actual, expected);
    }
}
