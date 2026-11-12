//! Content hashing used for change detection.

use sha2::{Digest, Sha256};

/// Lowercase hex SHA-256 of `bytes`.
pub fn sha256_hex(bytes: impl AsRef<[u8]>) -> String {
    hex::encode(Sha256::digest(bytes.as_ref()))
}

/// SHA-256 block size in bytes.
const BLOCK: usize = 64;

/// HMAC-SHA256 of `message` under `key` (RFC 2104), used to verify webhook
/// signatures.
pub fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    // A key longer than one block is replaced by its hash; a shorter one is
    // padded with zeros.
    let mut block = [0u8; BLOCK];
    if key.len() > BLOCK {
        block[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        block[..key.len()].copy_from_slice(key);
    }
    let mut inner = Sha256::new();
    inner.update(block.map(|b| b ^ 0x36));
    inner.update(message);
    let mut outer = Sha256::new();
    outer.update(block.map(|b| b ^ 0x5c));
    outer.update(inner.finalize());
    let mut out = [0u8; 32];
    out.copy_from_slice(&outer.finalize());
    out
}

/// Compare without leaking how many leading bytes matched.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Normalize text before hashing so that line-ending and trailing-whitespace
/// differences (git autocrlf, editor settings) are not reported as edits.
pub fn normalize_text(text: &str) -> String {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        out.push_str(line.trim_end());
        out.push('\n');
    }
    while out.ends_with("\n\n") {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_stable() {
        assert_eq!(
            sha256_hex("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    /// Test cases 1, 2, 6 and 7 of RFC 4231, section 4.
    #[test]
    fn hmac_matches_rfc_4231() {
        let hex_of = |key: &[u8], data: &[u8]| hex::encode(hmac_sha256(key, data));
        assert_eq!(
            hex_of(&[0x0b; 20], b"Hi There"),
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
        assert_eq!(
            hex_of(b"Jefe", b"what do ya want for nothing?"),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
        // Keys longer than the block size are hashed first.
        assert_eq!(
            hex_of(
                &[0xaa; 131],
                b"Test Using Larger Than Block-Size Key - Hash Key First"
            ),
            "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"
        );
        assert_eq!(
            hex_of(
                &[0xaa; 131],
                b"This is a test using a larger than block-size key and a larger than block-size data. The key needs to be hashed before being used by the HMAC algorithm."
            ),
            "9b09ffa71b942fcb27635fbcd5b0e944bfdc63644f0713938a7f51535c3a35e2"
        );
    }

    /// The example in GitHub's "Validating webhook deliveries" guide.
    #[test]
    fn hmac_matches_the_github_webhook_example() {
        assert_eq!(
            hex::encode(hmac_sha256(b"It's a Secret to Everybody", b"Hello, World!")),
            "757107ea0eb2509fc211221cce984b8a37570b6d7586c22c46f4379c8b043e17"
        );
    }

    #[test]
    fn constant_time_eq_compares_content_and_length() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"abcd"));
        assert!(constant_time_eq(b"", b""));
    }

    #[test]
    fn normalization_ignores_line_endings_and_trailing_space() {
        let unix = "# Title\n\nBody  \n";
        let windows = "\u{feff}# Title\r\n\r\nBody\r\n\r\n\r\n";
        assert_eq!(normalize_text(unix), normalize_text(windows));
        assert_eq!(normalize_text(unix), "# Title\n\nBody\n");
    }
}
