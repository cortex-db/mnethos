//! HTTP server exposing the Mnethos client authentication contract.

use std::sync::Arc;

use serde::Serialize;

use crate::error::ServerError;
use crate::store::UserStore;

/// A fully-rendered HTTP response (status code plus JSON body).
///
/// Produced by [`Server::handle`] so the routing logic can be unit-tested
/// without binding a network socket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    /// HTTP status code.
    pub status: u16,
    /// Serialized JSON response body.
    pub body: String,
}

impl HttpResponse {
    /// Builds a response by serializing `value` to JSON with the given status.
    fn json(status: u16, value: &impl Serialize) -> Self {
        let body = serde_json::to_string(value)
            .unwrap_or_else(|_| String::from(r#"{"error":"serialization_failed"}"#));
        Self { status, body }
    }

    /// Builds an error response with the given status and machine-readable
    /// `error` code.
    fn error(status: u16, code: &str) -> Self {
        Self { status, body: format!(r#"{{"error":"{code}"}}"#) }
    }
}

/// HTTP server backed by a [`UserStore`].
///
/// The server implements the two routes the Mnethos CLI depends on:
/// `GET /auth/user` and `GET /auth/usage`, both authenticated with an
/// `Authorization: Bearer <api_key>` header. A `GET /health` route is also
/// provided for readiness checks.
pub struct Server<S>(Arc<S>);

impl<S> Server<S> {
    /// Creates a new server over the given shared store.
    pub fn new(store: Arc<S>) -> Self {
        Self(store)
    }
}

impl<S: UserStore + 'static> Server<S> {
    /// Starts the blocking request loop, listening on `address`
    /// (e.g. `127.0.0.1:8080`).
    ///
    /// # Errors
    /// Returns [`ServerError::Bind`] when the listening socket cannot be bound.
    pub fn run(&self, address: &str) -> Result<(), ServerError> {
        let server = tiny_http::Server::http(address).map_err(|error| ServerError::Bind {
            address: address.to_string(),
            message: error.to_string(),
        })?;

        tracing::info!(address = %address, "Mnethos server listening");

        for request in server.incoming_requests() {
            self.serve(request);
        }

        Ok(())
    }

    /// Resolves a single request into an [`HttpResponse`].
    ///
    /// This is the pure core of the server: it has no I/O side effects and is
    /// driven both by the live request loop and by unit tests.
    pub fn handle(&self, method: &str, path: &str, authorization: Option<&str>) -> HttpResponse {
        let path = path.split('?').next().unwrap_or(path);
        let path = path.trim_end_matches('/');

        match (method, path) {
            ("GET", "" | "/health") => {
                HttpResponse::json(200, &serde_json::json!({"status": "ok"}))
            }
            ("GET", "/auth/user") => self.auth_user(authorization),
            ("GET", "/auth/usage") => self.auth_usage(authorization),
            ("GET", _) => HttpResponse::error(404, "not_found"),
            _ => HttpResponse::error(405, "method_not_allowed"),
        }
    }

    /// Handles `GET /auth/user`.
    fn auth_user(&self, authorization: Option<&str>) -> HttpResponse {
        let Some(token) = bearer_token(authorization) else {
            return HttpResponse::error(401, "missing_bearer_token");
        };

        match self.0.user(token) {
            Some(user) => HttpResponse::json(200, &user),
            None => HttpResponse::error(401, "invalid_api_key"),
        }
    }

    /// Handles `GET /auth/usage`.
    fn auth_usage(&self, authorization: Option<&str>) -> HttpResponse {
        let Some(token) = bearer_token(authorization) else {
            return HttpResponse::error(401, "missing_bearer_token");
        };

        match self.0.usage(token) {
            Some(usage) => HttpResponse::json(200, &usage),
            None => HttpResponse::error(401, "invalid_api_key"),
        }
    }

    /// Reads a request off the wire, dispatches it, and writes the response.
    fn serve(&self, request: tiny_http::Request) {
        let method = method_str(request.method());
        let url = request.url().to_string();
        let authorization = request
            .headers()
            .iter()
            .find(|header| header.field.equiv("Authorization"))
            .map(|header| header.value.as_str().to_string());

        let response = self.handle(method, &url, authorization.as_deref());

        let http_response = tiny_http::Response::from_string(response.body)
            .with_status_code(response.status)
            .with_header(json_content_type());

        if let Err(error) = request.respond(http_response) {
            tracing::warn!(error = %error, "failed to write response");
        }
    }
}

/// Extracts the token from an `Authorization: Bearer <token>` header value.
///
/// The scheme match is case-insensitive and surrounding whitespace is trimmed.
/// Returns `None` when the header is absent, not a bearer token, or empty.
fn bearer_token(authorization: Option<&str>) -> Option<&str> {
    let value = authorization?.trim();
    let (scheme, token) = value.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("Bearer") {
        return None;
    }
    let token = token.trim();
    if token.is_empty() { None } else { Some(token) }
}

/// Maps a `tiny_http` method to its canonical HTTP verb string.
fn method_str(method: &tiny_http::Method) -> &'static str {
    match method {
        tiny_http::Method::Get => "GET",
        tiny_http::Method::Head => "HEAD",
        tiny_http::Method::Post => "POST",
        tiny_http::Method::Put => "PUT",
        tiny_http::Method::Delete => "DELETE",
        tiny_http::Method::Connect => "CONNECT",
        tiny_http::Method::Options => "OPTIONS",
        tiny_http::Method::Trace => "TRACE",
        tiny_http::Method::Patch => "PATCH",
        tiny_http::Method::NonStandard(_) => "NON_STANDARD",
    }
}

/// Builds the `Content-Type: application/json` response header.
fn json_content_type() -> tiny_http::Header {
    tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
        .expect("static content-type header is valid")
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use serde_json::json;

    use super::*;
    use crate::store::InMemoryUserStore;

    fn fixture() -> Server<InMemoryUserStore> {
        Server::new(Arc::new(InMemoryUserStore::demo()))
    }

    #[test]
    fn test_health_route() {
        let actual = fixture().handle("GET", "/health", None);
        let expected = HttpResponse { status: 200, body: r#"{"status":"ok"}"#.to_string() };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_auth_user_with_valid_bearer() {
        let actual = fixture().handle("GET", "/auth/user", Some("Bearer mnethos-demo-key"));
        let expected_body = json!({ "authProviderId": "demo-user" });
        assert_eq!(actual.status, 200);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&actual.body).unwrap(),
            expected_body
        );
    }

    #[test]
    fn test_auth_usage_with_valid_bearer() {
        let actual = fixture().handle("GET", "/auth/usage", Some("Bearer mnethos-demo-key"));
        let expected_body = json!({
            "plan": { "type": "pro" },
            "usage": { "current": 10, "limit": 100, "remaining": 90 }
        });
        assert_eq!(actual.status, 200);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&actual.body).unwrap(),
            expected_body
        );
    }

    #[test]
    fn test_auth_user_missing_authorization_is_unauthorized() {
        let actual = fixture().handle("GET", "/auth/user", None);
        let expected = HttpResponse {
            status: 401,
            body: r#"{"error":"missing_bearer_token"}"#.to_string(),
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_auth_user_invalid_key_is_unauthorized() {
        let actual = fixture().handle("GET", "/auth/user", Some("Bearer wrong-key"));
        let expected = HttpResponse {
            status: 401,
            body: r#"{"error":"invalid_api_key"}"#.to_string(),
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_trailing_slash_and_query_are_normalized() {
        let actual = fixture().handle(
            "GET",
            "/auth/user/?foo=bar",
            Some("Bearer mnethos-demo-key"),
        );
        assert_eq!(actual.status, 200);
    }

    #[test]
    fn test_unknown_route_is_not_found() {
        let actual = fixture().handle("GET", "/nope", None);
        let expected = HttpResponse { status: 404, body: r#"{"error":"not_found"}"#.to_string() };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_non_get_method_is_rejected() {
        let actual = fixture().handle("POST", "/auth/user", Some("Bearer mnethos-demo-key"));
        let expected = HttpResponse {
            status: 405,
            body: r#"{"error":"method_not_allowed"}"#.to_string(),
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_bearer_token_parsing() {
        assert_eq!(bearer_token(Some("Bearer abc")), Some("abc"));
        assert_eq!(bearer_token(Some("bearer abc")), Some("abc"));
        assert_eq!(bearer_token(Some("  Bearer   abc  ")), Some("abc"));
        assert_eq!(bearer_token(Some("Basic abc")), None);
        assert_eq!(bearer_token(Some("Bearer ")), None);
        assert_eq!(bearer_token(None), None);
    }
}
