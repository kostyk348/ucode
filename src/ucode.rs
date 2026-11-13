//! The ucode itself: 128-bit chunks and the `urn:ucode:` string form of RFC 6588.

use std::fmt;
use std::str::FromStr;

/// Hex digits per 128-bit chunk.
pub const CHUNK_HEX: usize = 32;
/// Bytes per 128-bit chunk.
pub const CHUNK_BYTES: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UcodeError {
    /// Not a `urn:` at all.
    NotAnUrn(String),
    /// A `urn:` but not in the `ucode` namespace.
    WrongNamespace(String),
    /// `urn:ucode:` followed by nothing.
    MissingName,
    /// The mandatory underscore that keeps the name usable as an XML identifier.
    MissingUnderscore(String),
    /// The namespace is case-sensitive and only `A`..`F` are allowed.
    LowercaseHex(String),
    /// A character that is not a hexadecimal digit at all.
    BadHex { found: char, position: usize },
    /// The hex digits do not fill whole 128-bit chunks.
    BadLength { digits: usize },
    /// Raw bytes that do not fill a whole number of chunks.
    BadByteLength { bytes: usize },
}

impl fmt::Display for UcodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UcodeError::NotAnUrn(text) => write!(f, "{text:?} is not a URN"),
            UcodeError::WrongNamespace(text) => write!(f, "{text:?} is not in the ucode namespace"),
            UcodeError::MissingName => write!(f, "urn:ucode: names nothing (expected urn:ucode:_ and 32 hex digits)"),
            UcodeError::MissingUnderscore(rest) => {
                write!(f, "the name must start with an underscore: {:?} (RFC 6588 makes it an XML-safe identifier)", format!("_{rest}"))
            }
            UcodeError::LowercaseHex(text) => {
                write!(f, "{text:?}: the ucode namespace is case-sensitive, so hexadecimal must be uppercase")
            }
            UcodeError::BadHex { found, position } => write!(f, "{found:?} at position {position} is not a hexadecimal digit"),
            UcodeError::BadLength { digits } => {
                write!(f, "{digits} hex digits do not fill whole 128-bit chunks (32 digits each)")
            }
            UcodeError::BadByteLength { bytes } => write!(f, "{bytes} bytes do not fill whole 128-bit chunks (16 bytes each)"),
        }
    }
}

/// A ucode: one or more 128-bit chunks, because RFC 6588 defines the number as `1*ucode-value` —
/// the identifier is extensible in units of 128 bits, and a parser that accepts exactly 32 hex
/// digits will reject valid data one day.
#[derive(Clone, PartialEq, Eq)]
pub struct Ucode {
    chunks: Vec<[u8; CHUNK_BYTES]>,
}

impl Ucode {
    /// The short form, 128 bits.
    pub fn from_u128(value: u128) -> Self {
        Self { chunks: vec![value.to_be_bytes()] }
    }

    /// The value, if it is a single 128-bit chunk.
    pub fn as_u128(&self) -> Option<u128> {
        match self.chunks.as_slice() {
            [chunk] => Some(u128::from_be_bytes(*chunk)),
            _ => None,
        }
    }

    pub fn chunks(&self) -> usize {
        self.chunks.len()
    }

    pub fn chunk(&self, index: usize) -> Option<[u8; CHUNK_BYTES]> {
        self.chunks.get(index).copied()
    }

    /// Read hexadecimal, with or without `urn:ucode:` and with or without the leading underscore,
    /// in either case.
    ///
    /// This is the lenient door: the wild contains lowercase ucodes and ucodes copied without the
    /// prefix, and refusing them helps nobody. [`Ucode::parse_strict`] is the RFC door.
    pub fn from_hex(hex: &str) -> Result<Self, UcodeError> {
        let trimmed = hex.trim();
        let without_prefix = match trimmed.len() >= 10 && trimmed[..10].eq_ignore_ascii_case("urn:ucode:") {
            true => &trimmed[10..],
            false => trimmed,
        };
        let digits = without_prefix.strip_prefix('_').unwrap_or(without_prefix);
        Self::from_digits(digits, false)
    }

    /// Read the RFC form exactly: `urn:ucode:` + `_` + 32 or more uppercase hex digits.
    pub fn parse_strict(text: &str) -> Result<Self, UcodeError> {
        if !text.starts_with("urn:") {
            return Err(UcodeError::NotAnUrn(text.to_string()));
        }
        let rest = match text.strip_prefix("urn:ucode:") {
            Some(rest) => rest,
            None => return Err(UcodeError::WrongNamespace(text.to_string())),
        };
        if rest.is_empty() {
            return Err(UcodeError::MissingName);
        }
        let Some(digits) = rest.strip_prefix('_') else {
            return Err(UcodeError::MissingUnderscore(rest.to_string()));
        };
        if digits.is_empty() {
            return Err(UcodeError::MissingName);
        }
        Self::from_digits(digits, true)
    }

    fn from_digits(digits: &str, strict_case: bool) -> Result<Self, UcodeError> {
        if digits.len() % CHUNK_HEX != 0 {
            return Err(UcodeError::BadLength { digits: digits.len() });
        }
        if strict_case && digits.chars().any(|c| c.is_ascii_lowercase()) {
            return Err(UcodeError::LowercaseHex(digits.to_string()));
        }
        let mut chunks = Vec::with_capacity(digits.len() / CHUNK_HEX);
        for (index, chunk) in digits.as_bytes().chunks(CHUNK_HEX).enumerate() {
            let mut bytes = [0u8; CHUNK_BYTES];
            for (byte_index, pair) in chunk.chunks(2).enumerate() {
                let text = std::str::from_utf8(pair).unwrap_or("");
                match u8::from_str_radix(text, 16) {
                    Ok(value) => bytes[byte_index] = value,
                    Err(_) => {
                        let found = text.chars().next().unwrap_or('?');
                        let position = index * CHUNK_HEX + byte_index * 2;
                        return Err(UcodeError::BadHex { found, position });
                    }
                }
            }
            chunks.push(bytes);
        }
        Ok(Self { chunks })
    }

    /// Raw bytes: the plain big-endian value, 16 bytes per chunk.
    ///
    /// This is **not** a tag payload: the binary carrier is specified by the ucode Tag Architecture
    /// (UID-00017), which this crate does not claim to implement.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, UcodeError> {
        if bytes.len() % CHUNK_BYTES != 0 || bytes.is_empty() {
            return Err(UcodeError::BadByteLength { bytes: bytes.len() });
        }
        Ok(Self { chunks: bytes.chunks(CHUNK_BYTES).map(|chunk| {
            let mut out = [0u8; CHUNK_BYTES];
            out.copy_from_slice(chunk);
            out
        }).collect() })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.chunks.iter().flat_map(|chunk| chunk.to_vec()).collect()
    }

    /// The canonical string form.
    pub fn to_urn(&self) -> String {
        let mut out = String::with_capacity(10 + 1 + self.chunks.len() * CHUNK_HEX);
        out.push_str("urn:ucode:_");
        for chunk in &self.chunks {
            for byte in chunk {
                out.push_str(&format!("{byte:02X}"));
            }
        }
        out
    }
}

impl fmt::Display for Ucode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_urn())
    }
}

impl fmt::Debug for Ucode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ucode({})", self.to_urn())
    }
}

impl FromStr for Ucode {
    type Err = UcodeError;

    /// Strict, as the RFC reads: `urn:ucode:_` and uppercase hexadecimal.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Ucode::parse_strict(text)
    }
}
