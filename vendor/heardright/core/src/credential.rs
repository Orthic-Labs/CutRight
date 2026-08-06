//! Pure credential helpers.
//!
//! OS credential storage and random-byte generation stay in `src-tauri`; the
//! encoding helper lives here so its known-vector tests run outside the Tauri
//! test harness.

/// Standard base64 (with padding). Delegates to the `base64` crate — the
/// former hand-rolled encoder is gone (dispatch #12); the known-vector tests
/// below now guard the crate wiring.
pub fn base64_encode(bytes: &[u8]) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_known_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn thirty_two_bytes_encode_to_forty_four_chars() {
        let encoded = base64_encode(&[0u8; 32]);
        assert_eq!(encoded.len(), 44);
        assert!(encoded.ends_with('='));
    }
}
