//! ucode, the identifier that is only a number.
//!
//! The TRON project's ucode is a 128-bit number assigned to a thing, a place or even a concept, and
//! its designers made one decision that goes against almost every other identifier scheme: **the
//! number carries no meaning**. There are no sub-fields to parse, no prefixes that encode a
//! manufacturer, no version digit. Everything you want to know lives in a data server that the
//! number is *resolved* against — which is what makes it survive the death of the organisation that
//! issued it. `ucode is never recycled`; the binding is permanent.
//!
//! What is standardised and implemented here is the **string form**, from
//! [RFC 6588](https://www.rfc-editor.org/rfc/rfc6588.txt):
//!
//! ```text
//! UCODE-URN  = "urn:ucode:" ucode-name
//! ucode-name = "_" ucode-number
//! ucode-number = 1*ucode-value        ; one or more, because ucode is extensible in 128-bit units
//! ucode-value  = 32HEXDIG             ; uppercase only: "A".."F", never "a".."f"
//! ```
//!
//! ```
//! use ucode::Ucode;
//!
//! let id: Ucode = "urn:ucode:_0123456789ABCDEF0123456789ABCDEF".parse().unwrap();
//! assert_eq!(id.chunks(), 1);
//! assert_eq!(id.as_u128(), Some(0x0123_4567_89AB_CDEF_0123_4567_89AB_CDEF));
//! assert_eq!(id.to_string(), "urn:ucode:_0123456789ABCDEF0123456789ABCDEF");
//! ```
//!
//! Three details of the RFC are easy to get wrong, and the tests here pin all three:
//!
//! 1. the underscore after `ucode:` is mandatory — it exists so the name can also be an XML
//!    identifier, which may not start with a digit;
//! 2. hexadecimal is **uppercase only**, because the namespace is case-sensitive on purpose;
//! 3. `ucode-number` is `1*ucode-value`, so a future 256-bit or 384-bit ucode is a valid URN, and a
//!    parser that insists on exactly 32 hex digits will reject valid data one day.
//!
//! What this crate does not do: it does not issue ucodes (the Ubiquitous ID Center allocates them),
//! it does not resolve them (that is the Simplified ucode Resolution Protocol, UID-00005), and it
//! does not guess at a tag layout — the RFC leaves the binary carrier to the ucode Tag Architecture,
//! so [`Ucode::to_bytes`] gives you the plain 128-bit value and says so, rather than pretending to
//! produce a compliant NFC payload.

pub mod ident;
pub mod ucode;

pub use ident::{Ident, IdentError};
pub use ucode::{Ucode, UcodeError};
