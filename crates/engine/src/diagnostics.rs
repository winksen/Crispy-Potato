//! Fixed public diagnostics. Never format database or transport error messages.
use std::error::Error;

pub fn cause(mut error: &(dyn Error + 'static)) -> &'static str {
    for _ in 0..8 {
        if let Some(error) = error.downcast_ref::<sqlx::Error>() {
            return match error {
                sqlx::Error::Database(db) => match db.code().as_deref() {
                    Some(code) if code.starts_with("28") => "database authentication failed",
                    Some("42501") => "database permission denied",
                    Some("57014" | "55P03") => "database timeout",
                    Some(code) if code.starts_with("23") => "database constraint violation",
                    Some(code) if code.starts_with("08") || code.starts_with("57P0") => {
                        "database unreachable"
                    }
                    _ => "database query failed",
                },
                sqlx::Error::Io(_) | sqlx::Error::PoolClosed => "database unreachable",
                sqlx::Error::PoolTimedOut => "database timeout",
                sqlx::Error::Tls(_) => "database TLS failed",
                sqlx::Error::Configuration(_) => "database configuration is invalid",
                sqlx::Error::RowNotFound => "database record not found",
                _ => "database operation failed",
            };
        }
        if error.is::<serde_json::Error>() {
            return "invalid JSON data";
        }
        if error.is::<crate::http::Error>() {
            return "HTTP client failed";
        }
        if error.is::<std::io::Error>() {
            return "file operation failed";
        }
        match error.source() {
            Some(source) => error = source,
            None => break,
        }
    }
    "operation failed"
}

pub fn connection_cause(error: &sqlx::Error) -> &'static str {
    // SQLx retries connection refusal until acquisition times out. At startup
    // no connection has been acquired, so this is an unreachable database.
    if matches!(error, sqlx::Error::PoolTimedOut) {
        "database unreachable (connection timed out)"
    } else {
        cause(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_never_format_error_payloads() {
        let secret = "postgres://private:do-not-print@private-host/database";
        let error = sqlx::Error::Io(std::io::Error::other(secret));
        assert_eq!(cause(&error), "database unreachable");
        let error = sqlx::Error::Configuration(secret.into());
        assert_eq!(cause(&error), "database configuration is invalid");
        assert_eq!(cause(&sqlx::Error::PoolTimedOut), "database timeout");
        assert!(connection_cause(&sqlx::Error::PoolTimedOut).contains("unreachable"));
    }
}
