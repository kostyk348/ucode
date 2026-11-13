//! One canonical form for the four ways we say "this thing".
//!
//! A DOI, an arXiv id, a handle, an ISBN and an IPFS CID all name a thing, and all of them arrive in
//! half a dozen shapes (`doi:10.1000/xyz`, `https://doi.org/10.1000/xyz`, `http://dx.doi.org/...`).
//! A ucode names a thing too, and it is the only one of the group that carries no meaning at all —
//! which is exactly what makes it worth putting beside the others: what a ucode does by design
//! (survive the death of its issuer, be resolved rather than parsed) is what the rest only promise.
//!
//! This module is the boring, useful part: recognise an identifier, say what it is, and give one
//! canonical string per referent, so that "the same paper" has one key in a database instead of six.

use std::fmt;

use crate::ucode::Ucode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentError {
    /// Nothing in the input looked like an identifier this crate knows.
    Unknown(String),
    /// It looked like one, and was not.
    Invalid { kind: &'static str, reason: String },
}

impl fmt::Display for IdentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdentError::Unknown(text) => write!(
                f,
                "{text:?} is not an identifier this tool knows (ucode, DOI, arXiv, handle, ISBN, IPFS CID)"
            ),
            IdentError::Invalid { kind, reason } => write!(f, "not a valid {kind}: {reason}"),
        }
    }
}

/// A recognised identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ident {
    /// The TRON/Ubiquitous ID identifier: a number with no meaning of its own.
    Ucode(Ucode),
    /// A DOI, stored lowercase: DOIs are case-insensitive, and a key that matches is worth more
    /// than a key that preserves the publisher's capitalisation.
    Doi(String),
    /// An arXiv id, with the version if one was given.
    Arxiv { id: String, version: Option<u32> },
    /// A Handle (the DOI's ancestor), prefix and suffix.
    Handle { prefix: String, suffix: String },
    /// An ISBN, always the thirteen-digit form.
    Isbn(String),
    /// A CIDv1 in base32 — content address, the identifier of a thing by what it is made of.
    Cidv1 { cid: String, codec: u64 },
}

impl Ident {
    pub fn parse(input: &str) -> Result<Self, IdentError> {
        let text = input.trim();
        if text.is_empty() {
            return Err(IdentError::Unknown(input.to_string()));
        }
        if let Some(rest) = strip_prefix_ci(text, "urn:ucode:") {
            return Ucode::parse_strict(&format!("urn:ucode:{rest}"))
                .map(Ident::Ucode)
                .map_err(|error| IdentError::Invalid { kind: "ucode", reason: error.to_string() });
        }
        if let Some(rest) = strip_prefix_ci(text, "urn:isbn:") {
            return parse_isbn(rest).map(Ident::Isbn);
        }
        for prefix in ["doi:", "https://doi.org/", "http://doi.org/", "https://dx.doi.org/", "http://dx.doi.org/", "http://www.doi.org/", "https://www.doi.org/"] {
            if let Some(rest) = strip_prefix_ci(text, prefix) {
                return parse_doi(rest).map(Ident::Doi);
            }
        }
        for prefix in ["arxiv:", "https://arxiv.org/abs/", "http://arxiv.org/abs/", "https://arxiv.org/pdf/", "http://arxiv.org/pdf/", "https://export.arxiv.org/abs/"] {
            if let Some(rest) = strip_prefix_ci(text, prefix) {
                return parse_arxiv(rest).map(|(id, version)| Ident::Arxiv { id, version });
            }
        }
        for prefix in ["hdl:", "https://hdl.handle.net/", "http://hdl.handle.net/"] {
            if let Some(rest) = strip_prefix_ci(text, prefix) {
                return parse_handle(rest).map(|(prefix, suffix)| Ident::Handle { prefix, suffix });
            }
        }
        for prefix in ["ipfs://", "https://ipfs.io/ipfs/", "http://ipfs.io/ipfs/"] {
            if let Some(rest) = strip_prefix_ci(text, prefix) {
                return parse_cid(rest).map(|(cid, codec)| Ident::Cidv1 { cid, codec });
            }
        }
        for prefix in ["isbn:", "isbn-13:", "isbn-10:"] {
            if let Some(rest) = strip_prefix_ci(text, prefix) {
                return parse_isbn(rest).map(Ident::Isbn);
            }
        }
        // A bare ISBN is what you get when someone copies a number off a book: ten or thirteen
        // digits, possibly with hyphens. Tried before the CID/ucode fallbacks because neither can
        // be ten or thirteen characters long.
        let digit_count = text.chars().filter(char::is_ascii_digit).count();
        let only_isbn_characters = text.chars().all(|c| c.is_ascii_digit() || matches!(c, '-' | ' ' | 'X' | 'x'));
        if only_isbn_characters && matches!(digit_count, 10 | 13) {
            if let Ok(isbn) = parse_isbn(text) {
                return Ok(Ident::Isbn(isbn));
            }
        }
        // A bare CID is common in IPFS tooling, and a bare ucode is common in Japanese data.
        if let Ok((cid, codec)) = parse_cid(text) {
            return Ok(Ident::Cidv1 { cid, codec });
        }
        if let Ok(ucode) = Ucode::from_hex(text) {
            return Ok(Ident::Ucode(ucode));
        }
        Err(IdentError::Unknown(input.to_string()))
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Ident::Ucode(_) => "ucode",
            Ident::Doi(_) => "DOI",
            Ident::Arxiv { .. } => "arXiv",
            Ident::Handle { .. } => "handle",
            Ident::Isbn(_) => "ISBN",
            Ident::Cidv1 { .. } => "IPFS CID",
        }
    }

    /// One string per referent: the key to store, compare and deduplicate.
    pub fn canonical(&self) -> String {
        match self {
            Ident::Ucode(ucode) => ucode.to_urn(),
            Ident::Doi(doi) => format!("doi:{doi}"),
            Ident::Arxiv { id, version } => match version {
                Some(version) => format!("arXiv:{id}v{version}"),
                None => format!("arXiv:{id}"),
            },
            Ident::Handle { prefix, suffix } => format!("hdl:{prefix}/{suffix}"),
            Ident::Isbn(isbn) => format!("urn:isbn:{isbn}"),
            Ident::Cidv1 { cid, .. } => format!("ipfs://{cid}"),
        }
    }

    /// Where a browser should go, when there is somewhere to go. A ucode has no public resolver
    /// here on purpose: resolution belongs to the ucode Resolution Protocol and to servers that
    /// speak it, and inventing a URL would be a promise this crate cannot keep.
    pub fn url(&self) -> Option<String> {
        match self {
            Ident::Ucode(_) => None,
            Ident::Doi(doi) => Some(format!("https://doi.org/{doi}")),
            Ident::Arxiv { id, version } => Some(match version {
                Some(version) => format!("https://arxiv.org/abs/{id}v{version}"),
                None => format!("https://arxiv.org/abs/{id}"),
            }),
            Ident::Handle { prefix, suffix } => Some(format!("https://hdl.handle.net/{prefix}/{suffix}")),
            Ident::Isbn(isbn) => Some(format!("https://search.worldcat.org/isbn/{isbn}")),
            Ident::Cidv1 { cid, .. } => Some(format!("https://ipfs.io/ipfs/{cid}")),
        }
    }

    /// What a caller should know about how the input was changed to make the canonical form: no
    /// silences, so a database migration can be reviewed.
    pub fn notes(&self) -> Vec<String> {
        match self {
            Ident::Isbn(isbn) if isbn.starts_with("978") => vec!["ISBN was normalised to the thirteen-digit form".into()],
            Ident::Doi(_) => vec!["DOIs are case-insensitive; the canonical form is lowercase".into()],
            Ident::Arxiv { version: None, .. } => vec!["no version given: the canonical form keeps it that way rather than assuming v1".into()],
            Ident::Ucode(ucode) if ucode.chunks() > 1 => vec![format!(
                "this ucode is {} chunks ({} bits): RFC 6588 allows an identifier to grow in 128-bit units",
                ucode.chunks(),
                ucode.chunks() * 128
            )],
            Ident::Cidv1 { codec, .. } => {
                let name = codec_name(*codec);
                match name {
                    "raw" => vec!["a raw block: the bytes are the content".into()],
                    other => vec![format!("{other} codec: this address names a node, not the bytes themselves")],
                }
            }
            _ => Vec::new(),
        }
    }
}

fn strip_prefix_ci<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    if text.len() >= prefix.len() && text[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(&text[prefix.len()..])
    } else {
        None
    }
}

/// `10.` + four to nine digits + `/` + something that is not whitespace.
fn parse_doi(rest: &str) -> Result<String, IdentError> {
    let doi = rest.trim();
    if doi.is_empty() {
        return Err(invalid("DOI", "nothing after the prefix"));
    }
    let Some(after_dot) = doi.strip_prefix("10.") else {
        return Err(invalid("DOI", format!("{doi:?} does not start with the registrant prefix 10.")));
    };
    let Some((registrant, suffix)) = after_dot.split_once('/') else {
        return Err(invalid("DOI", format!("{doi:?} has no suffix after the registrant")));
    };
    if !(4..=9).contains(&registrant.len()) || !registrant.chars().all(|c| c.is_ascii_digit()) {
        return Err(invalid("DOI", format!("registrant {registrant:?} is not four to nine digits")));
    }
    if suffix.is_empty() || suffix.chars().any(char::is_whitespace) {
        return Err(invalid("DOI", format!("suffix {suffix:?} is empty or contains whitespace")));
    }
    Ok(doi.to_lowercase())
}

/// `NNNN.NNNNN` (new style) or `archive[.SC]/YYMMNNN` (old style), with an optional `vN`.
fn parse_arxiv(rest: &str) -> Result<(String, Option<u32>), IdentError> {
    let mut id = rest.trim().trim_end_matches(".pdf").to_string();
    if id.is_empty() {
        return Err(invalid("arXiv id", "nothing after the prefix"));
    }
    let mut version = None;
    if let Some(index) = id.rfind('v') {
        let candidate = &id[index + 1..];
        if !candidate.is_empty() && candidate.chars().all(|c| c.is_ascii_digit()) && id[..index].contains(['.', '/']) {
            version = candidate.parse().ok();
            id.truncate(index);
        }
    }
    let new_style = id.len() >= 9 && id.split_once('.').is_some_and(|(left, right)| {
        left.len() >= 4 && left.chars().all(|c| c.is_ascii_digit()) && right.len() >= 4 && right.chars().all(|c| c.is_ascii_digit())
    });
    let old_style = id.split_once('/').is_some_and(|(archive, number)| {
        !archive.is_empty()
            && number.len() == 7
            && number.chars().all(|c| c.is_ascii_digit())
            && archive.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
    });
    if !new_style && !old_style {
        return Err(invalid("arXiv id", format!("{id:?} is neither NNNN.NNNNN nor archive/YYMMNNN")));
    }
    Ok((id, version))
}

fn parse_handle(rest: &str) -> Result<(String, String), IdentError> {
    let handle = rest.trim();
    let Some((prefix, suffix)) = handle.split_once('/') else {
        return Err(invalid("handle", format!("{handle:?} has no suffix after the prefix")));
    };
    if prefix.is_empty()
        || !prefix.chars().next().is_some_and(|c| c.is_ascii_digit())
        || !prefix.chars().all(|c| c.is_ascii_digit() || c == '.')
    {
        return Err(invalid("handle", format!("prefix {prefix:?} is not a numeric handle prefix")));
    }
    if suffix.is_empty() || suffix.chars().any(char::is_whitespace) {
        return Err(invalid("handle", format!("suffix {suffix:?} is empty or contains whitespace")));
    }
    Ok((prefix.to_string(), suffix.to_string()))
}

/// Ten or thirteen digits (hyphens allowed on input), checksum verified, always thirteen out.
fn parse_isbn(rest: &str) -> Result<String, IdentError> {
    let digits: String = rest.chars().filter(|c| *c != '-' && *c != ' ').collect();
    match digits.len() {
        10 => {
            if !isbn10_is_valid(&digits) {
                return Err(invalid("ISBN", format!("{rest:?} has a wrong ISBN-10 check digit")));
            }
            let body = format!("978{}", &digits[..9]);
            let check = isbn13_check_digit(&body);
            Ok(format!("{body}{check}"))
        }
        13 => {
            if !digits.chars().all(|c| c.is_ascii_digit()) {
                return Err(invalid("ISBN", format!("{rest:?} contains something that is not a digit")));
            }
            let expected = isbn13_check_digit(&digits[..12]);
            if digits.as_bytes()[12] as char != expected {
                return Err(invalid("ISBN", format!("{rest:?} has a wrong ISBN-13 check digit (expected {expected})")));
            }
            Ok(digits)
        }
        other => Err(invalid("ISBN", format!("{other} digits: an ISBN has ten or thirteen"))),
    }
}

fn isbn13_check_digit(first12: &str) -> char {
    let sum: u32 = first12
        .chars()
        .enumerate()
        .map(|(index, c)| {
            let digit = c.to_digit(10).unwrap_or(0);
            if index % 2 == 0 { digit } else { digit * 3 }
        })
        .sum();
    char::from_digit((10 - sum % 10) % 10, 10).unwrap_or('0')
}

/// An ISBN-10 is valid when the weighted sum of all ten characters is divisible by eleven, with a
/// final `X` counting as ten. Validating the sum is the whole check — computing a "check digit" from
/// ten characters that already include it is the classic way to reject every valid ISBN.
fn isbn10_is_valid(digits: &str) -> bool {
    let mut sum = 0u32;
    for (index, c) in digits.chars().enumerate() {
        let value = match c {
            'X' | 'x' if index == 9 => 10,
            _ => match c.to_digit(10) {
                Some(value) => value,
                None => return false,
            },
        };
        sum += value * (10 - index as u32);
    }
    sum % 11 == 0
}

/// A CIDv1 in base32: `b` + RFC 4648 lowercase base32. The bytes are checked as
/// `<version><codec><multihash>`, because a string that decodes is not automatically a CID, and a
/// CID that names a dag-pb node is not the same thing as a raw block with the same bytes.
fn parse_cid(rest: &str) -> Result<(String, u64), IdentError> {
    let cid = rest.trim().trim_end_matches('/');
    let Some(body) = cid.strip_prefix('b') else {
        return Err(invalid("CID", format!("{cid:?} is not base32 CIDv1 (the multibase prefix must be 'b')")));
    };
    if body.len() < 8 {
        return Err(invalid("CID", format!("{cid:?} is too short to be a CIDv1")));
    }
    let bytes = base32_decode(body).map_err(|reason| invalid("CID", reason))?;
    let mut at = 0usize;
    let version = read_varint(&bytes, &mut at).map_err(|reason| invalid("CID", reason))?;
    if version != 1 {
        return Err(invalid("CID", format!("version {version} is not CIDv1 (multibase strings of CIDv0 do not start with 'b')")));
    }
    let codec = read_varint(&bytes, &mut at).map_err(|reason| invalid("CID", reason))?;
    let hash_code = read_varint(&bytes, &mut at).map_err(|reason| invalid("CID", reason))?;
    let digest_length = read_varint(&bytes, &mut at).map_err(|reason| invalid("CID", reason))?;
    if hash_code != 0x12 {
        return Err(invalid("CID", format!("multihash code {hash_code:#04X} is not sha2-256 (0x12)")));
    }
    if digest_length != 32 {
        return Err(invalid("CID", format!("a sha2-256 digest is 32 bytes, not {digest_length}")));
    }
    if bytes.len() != at + 32 {
        return Err(invalid("CID", format!("{} bytes after the digest, expected none", bytes.len().saturating_sub(at + 32))));
    }
    Ok((cid.to_string(), codec))
}

/// Unsigned LEB128, the varint of multiformats.
fn read_varint(bytes: &[u8], at: &mut usize) -> Result<u64, String> {
    let mut value = 0u64;
    let mut shift = 0u32;
    loop {
        let Some(byte) = bytes.get(*at) else {
            return Err("truncated varint".to_string());
        };
        *at += 1;
        value |= u64::from(byte & 0x7F) << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
        shift += 7;
        if shift > 63 {
            return Err("varint is longer than 64 bits".to_string());
        }
    }
}

/// The multiformats codec names worth recognising in a note.
pub fn codec_name(codec: u64) -> &'static str {
    match codec {
        0x55 => "raw",
        0x70 => "dag-pb",
        0x71 => "dag-cbor",
        0x72 => "libp2p-key",
        _ => "unknown codec",
    }
}

fn base32_decode(body: &str) -> Result<Vec<u8>, String> {
    const ALPHABET: &[u8; 32] = b"abcdefghijklmnopqrstuvwxyz234567";
    let mut bits: u32 = 0;
    let mut collected = 0u32;
    let mut out = Vec::new();
    for c in body.chars() {
        let c = c.to_ascii_lowercase();
        let value = ALPHABET.iter().position(|a| *a == c as u8).ok_or_else(|| format!("{c:?} is not a base32 digit"))?;
        bits = (bits << 5) | value as u32;
        collected += 5;
        if collected >= 8 {
            collected -= 8;
            out.push((bits >> collected) as u8);
        }
    }
    Ok(out)
}

fn invalid(kind: &'static str, reason: impl Into<String>) -> IdentError {
    IdentError::Invalid { kind, reason: reason.into() }
}
