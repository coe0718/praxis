//! Secret zeroization helpers.
//!
//! Wraps sensitive strings so their bytes are explicitly overwritten
//! with zeros when dropped.  Uses the `zeroize` crate.

use zeroize::{Zeroize, ZeroizeOnDrop};

/// A wrapper around a `String` that zeroizes its contents on drop.
#[derive(Debug, Clone, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct SecretString(String);

impl SecretString {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn expose(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        let s = self.0.clone();
        // The Drop impl will zeroize the original.
        s
    }
}

impl From<String> for SecretString {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for SecretString {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// Zeroize a mutable string in place.
pub fn zeroize_string(s: &mut String) {
    s.zeroize();
}

/// Zeroize a mutable byte slice in place.
pub fn zeroize_bytes(b: &mut [u8]) {
    b.zeroize();
}
