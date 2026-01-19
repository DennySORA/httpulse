mod client;
mod helpers;

pub use client::ProbeClient;

use curl::easy::{Easy, HttpVersion, SslVersion};
use std::sync::OnceLock;
use std::time::Duration;

/// Cached result of TLS 1.3 support detection
static TLS13_SUPPORTED: OnceLock<bool> = OnceLock::new();

/// Known public endpoint that supports TLS 1.3 for testing.
const TLS13_TEST_URL: &str = "https://www.google.com";

/// Detects if the system's curl/SSL library supports TLS 1.3.
/// The result is cached after the first call.
///
/// This performs an actual TLS 1.3 connection test because some SSL backends
/// (e.g., SecureTransport on macOS) accept the API configuration but fail
/// at runtime when attempting TLS 1.3 handshakes.
pub fn detect_tls13_support() -> bool {
    *TLS13_SUPPORTED.get_or_init(|| {
        let mut easy = Easy::new();

        // First check if the API accepts TLS 1.3 configuration
        if easy
            .ssl_min_max_version(SslVersion::Tlsv13, SslVersion::Tlsv13)
            .is_err()
        {
            return false;
        }

        // API accepts it, but we need to verify runtime support
        // by actually attempting a TLS 1.3 connection
        if let Ok(supported) = test_tls13_connection() {
            return supported;
        }

        // If the test fails (e.g., network unavailable), fall back to false
        // to avoid runtime errors
        false
    })
}

/// Attempts an actual TLS 1.3 connection to verify runtime support.
fn test_tls13_connection() -> Result<bool, curl::Error> {
    let mut easy = Easy::new();

    easy.url(TLS13_TEST_URL)?;
    easy.ssl_min_max_version(SslVersion::Tlsv13, SslVersion::Tlsv13)?;
    easy.http_version(HttpVersion::V2TLS)?;
    easy.timeout(Duration::from_secs(5))?;
    easy.connect_timeout(Duration::from_secs(5))?;
    easy.nobody(true)?;
    easy.follow_location(false)?;

    match easy.perform() {
        Ok(()) => Ok(true),
        Err(err) => {
            let msg = err.to_string().to_ascii_lowercase();
            // If the error is TLS-related, TLS 1.3 is not supported
            if err.is_ssl_connect_error()
                || err.is_ssl_cacert()
                || err.is_ssl_certproblem()
                || err.is_ssl_cipher()
                || msg.contains("ssl")
                || msg.contains("tls")
                || msg.contains("handshake")
                || msg.contains("protocol")
            {
                Ok(false)
            } else {
                // Other errors (timeout, network) - cannot determine support
                Err(err)
            }
        }
    }
}
