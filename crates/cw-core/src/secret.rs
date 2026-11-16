//! Secrets kept in a settings file: API keys and the token of a remote
//! backend.
//!
//! On Windows a secret is encrypted with the Data Protection API for the
//! user who runs the program, so the file is of no use to another account or
//! on another machine. The stored form is the encrypted bytes as hexadecimal
//! text. Other systems have no such store: there a secret cannot be saved and
//! comes from an environment variable only.

use crate::{Error, Result};

/// A secret shorter than this shows no hint: its last characters would be
/// too large a part of it.
const HINT_FLOOR: usize = 12;
/// Characters of a secret's end that the hint shows.
const HINT_CHARS: usize = 4;

/// Whether this system can save a secret.
pub fn available() -> bool {
    cfg!(windows)
}

/// Encrypt `plain` for the current user and return the text to store.
pub fn protect(plain: &str) -> Result<String> {
    if plain.is_empty() {
        return Err(Error::invalid("a secret cannot be empty"));
    }
    platform::protect(plain.as_bytes()).map(hex::encode)
}

/// The secret behind text that [`protect`] returned, when the same user on
/// the same machine asks.
pub fn unprotect(stored: &str) -> Result<String> {
    let bytes = hex::decode(stored.trim())
        .map_err(|_| Error::invalid("the saved secret is not in the expected form"))?;
    if bytes.is_empty() {
        return Err(Error::invalid("the saved secret is empty"));
    }
    let plain = platform::unprotect(&bytes)?;
    String::from_utf8(plain).map_err(|_| Error::invalid("the saved secret is not text"))
}

/// The end of a secret, shown so that its owner can tell which one is saved.
/// Empty for a short secret.
pub fn hint(plain: &str) -> String {
    let count = plain.chars().count();
    if count < HINT_FLOOR {
        return String::new();
    }
    plain.chars().skip(count - HINT_CHARS).collect()
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod platform {
    use std::ptr;

    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
    };

    use crate::{Error, Result};

    /// Mixed into the encryption, so that another program's call with the
    /// same user does not decrypt a CodeWiki secret by accident. It is not a
    /// key: the user's account is what protects the secret.
    const ENTROPY: &[u8] = b"CodeWiki settings v1";

    fn blob(bytes: &[u8]) -> Result<CRYPT_INTEGER_BLOB> {
        let length = u32::try_from(bytes.len())
            .map_err(|_| Error::invalid("the secret is too long to protect"))?;
        Ok(CRYPT_INTEGER_BLOB {
            cbData: length,
            // The functions below only read their input.
            pbData: bytes.as_ptr().cast_mut(),
        })
    }

    fn failed(operation: &str) -> Error {
        Error::external(operation, std::io::Error::last_os_error())
    }

    /// Copy what the system returned, then give its memory back.
    fn take(output: CRYPT_INTEGER_BLOB) -> Vec<u8> {
        if output.pbData.is_null() {
            return Vec::new();
        }
        // SAFETY: the call that filled `output` succeeded, so `pbData` points
        // to `cbData` bytes that the system allocated for the caller.
        let bytes =
            unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
        // SAFETY: the system allocated `pbData` with `LocalAlloc` and asks
        // the caller to free it with `LocalFree`; it is not used again.
        unsafe { LocalFree(output.pbData.cast()) };
        bytes
    }

    pub fn protect(plain: &[u8]) -> Result<Vec<u8>> {
        let input = blob(plain)?;
        let entropy = blob(ENTROPY)?;
        let mut output = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: ptr::null_mut(),
        };
        // SAFETY: `input` and `entropy` point to slices that live through the
        // call, `output` is a valid place for the result, and the pointers
        // left null are optional. No window is asked for.
        let done = unsafe {
            CryptProtectData(
                &input,
                ptr::null(),
                &entropy,
                ptr::null(),
                ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        };
        if done == 0 {
            return Err(failed("could not protect the secret"));
        }
        Ok(take(output))
    }

    pub fn unprotect(stored: &[u8]) -> Result<Vec<u8>> {
        let input = blob(stored)?;
        let entropy = blob(ENTROPY)?;
        let mut output = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: ptr::null_mut(),
        };
        // SAFETY: as in `protect`; the description is not asked for.
        let done = unsafe {
            CryptUnprotectData(
                &input,
                ptr::null_mut(),
                &entropy,
                ptr::null(),
                ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        };
        if done == 0 {
            return Err(failed(
                "could not read the saved secret; it was saved by another user or on another machine",
            ));
        }
        Ok(take(output))
    }
}

#[cfg(not(windows))]
mod platform {
    use crate::{Error, Result};

    fn unavailable() -> Error {
        Error::invalid(
            "secrets cannot be saved on this system; set the environment variable instead",
        )
    }

    pub fn protect(_plain: &[u8]) -> Result<Vec<u8>> {
        Err(unavailable())
    }

    pub fn unprotect(_stored: &[u8]) -> Result<Vec<u8>> {
        Err(unavailable())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Made-up values: nothing here is a real key.
    const SAMPLE: &str = "sample-secret-for-tests-0042";

    #[cfg(windows)]
    #[test]
    fn a_protected_secret_reads_back_and_is_not_stored_in_clear() {
        assert!(available());
        let stored = protect(SAMPLE).unwrap();
        assert!(stored.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(!stored.contains(SAMPLE));
        assert!(!stored.contains(&hex::encode(SAMPLE)));
        assert_eq!(unprotect(&stored).unwrap(), SAMPLE);
        // Text beyond ASCII survives too.
        let stored = protect("clé-secrète-été-0042").unwrap();
        assert_eq!(unprotect(&stored).unwrap(), "clé-secrète-été-0042");
    }

    #[cfg(windows)]
    #[test]
    fn what_was_not_protected_here_does_not_read_back() {
        // Hexadecimal, but not something the system encrypted.
        assert!(unprotect("00112233445566778899aabbccddeeff").is_err());
        // A stored secret with one byte changed.
        let mut stored = protect(SAMPLE).unwrap();
        let last = if stored.ends_with('0') { "1" } else { "0" };
        stored.replace_range(stored.len() - 1.., last);
        assert!(unprotect(&stored).is_err());
    }

    #[cfg(not(windows))]
    #[test]
    fn a_system_without_a_store_saves_nothing() {
        assert!(!available());
        assert!(protect(SAMPLE).is_err());
        assert!(unprotect("00112233").is_err());
    }

    #[test]
    fn empty_and_malformed_input_is_refused() {
        assert!(protect("").is_err());
        assert!(unprotect("").is_err());
        assert!(unprotect("not hexadecimal").is_err());
    }

    #[test]
    fn the_hint_is_the_end_of_a_long_secret_only() {
        assert_eq!(hint(SAMPLE), "0042");
        assert_eq!(hint("exactly-12ch"), "12ch");
        // Too short: four characters would be a third of it or more.
        assert_eq!(hint("short-token"), "");
        assert_eq!(hint(""), "");
        // Counted in characters, not bytes.
        assert_eq!(hint("ééééééééééééabcd"), "abcd");
    }
}
