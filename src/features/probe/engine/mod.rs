mod client;
mod helpers;

pub use client::ProbeClient;

use curl::easy::{Easy, SslVersion};
use std::sync::OnceLock;

/// Cached result of TLS 1.3 support detection
static TLS13_SUPPORTED: OnceLock<bool> = OnceLock::new();

/// Detects if the system's curl/SSL library supports TLS 1.3.
/// The result is cached after the first call.
pub fn detect_tls13_support() -> bool {
    *TLS13_SUPPORTED.get_or_init(|| {
        let mut easy = Easy::new();

        // Try to set TLS 1.3 as min/max version
        // If the SSL backend doesn't support it, this will fail
        easy.ssl_min_max_version(SslVersion::Tlsv13, SslVersion::Tlsv13)
            .is_ok()
    })
}
