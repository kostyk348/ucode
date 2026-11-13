//! ucode per RFC 6588, and the naming layer that puts six identifier schemes in one form.

use ucode::ident::{codec_name, Ident};
use ucode::Ucode;

#[test]
fn the_rfc_example_is_the_canonical_form() {
    let text = "urn:ucode:_0123456789ABCDEF0123456789ABCDEF";
    let code: Ucode = text.parse().unwrap();
    assert_eq!(code.chunks(), 1);
    assert_eq!(code.as_u128(), Some(0x0123_4567_89AB_CDEF_0123_4567_89AB_CDEF));
    assert_eq!(code.to_string(), text, "the RFC writes this exact example");
}

#[test]
fn a_ucode_can_grow_in_128_bit_units() {
    // RFC 6588: ucode-number = 1*ucode-value, concatenated — so a 256-bit ucode is 64 hex digits.
    // A parser that insists on 32 will reject valid data one day.
    let text = "urn:ucode:_0123456789ABCDEF0123456789ABCDEFFEDCBA9876543210FEDCBA9876543210";
    let code = Ucode::from_hex(text).unwrap();
    assert_eq!(code.chunks(), 2);
    assert_eq!(code.as_u128(), None, "two chunks are not one u128");
    assert_eq!(code.to_urn(), text);
    let ident = Ident::parse(&text).unwrap();
    assert_eq!(ident.kind(), "ucode");
    assert!(ident.notes().iter().any(|note| note.contains("256 bits")), "{:?}", ident.notes());
}

#[test]
fn lowercase_hexadecimal_is_refused_by_the_strict_door_and_normalised_by_the_other() {
    // RFC 6588: "the entire UCODE-URN is case-sensitive ... only capital A..F are allowed".
    let lowercase = "urn:ucode:_0123456789abcdef0123456789abcdef";
    let error = lowercase.parse::<Ucode>().unwrap_err();
    assert!(error.to_string().contains("uppercase"), "{error}");
    // The lenient door is for data found in the wild, and it produces the canonical form.
    let code = Ucode::from_hex("0123456789abcdef0123456789abcdef").unwrap();
    assert_eq!(code.to_urn(), "urn:ucode:_0123456789ABCDEF0123456789ABCDEF");
}

#[test]
fn the_underscore_and_the_chunk_length_are_both_part_of_the_syntax() {
    // The underscore exists so the name can also be an XML identifier, which may not start with a
    // digit — drop it and the URN stops being one.
    let error = "urn:ucode:0123456789ABCDEF0123456789ABCDEF".parse::<Ucode>().unwrap_err();
    assert!(error.to_string().contains("underscore"), "{error}");

    let error = "urn:ucode:_0123456789ABCDEF0123456789ABCDE".parse::<Ucode>().unwrap_err();
    assert!(error.to_string().contains("31 hex digits"), "{error}");

    assert!("urn:ucode:_".parse::<Ucode>().is_err(), "a name with no digits");
    assert!("urn:foo:_0123456789ABCDEF0123456789ABCDEF".parse::<Ucode>().is_err(), "another namespace");
    assert!("0123456789ABCDEF0123456789ABCDEF".parse::<Ucode>().is_err(), "the URN prefix is not optional in the strict door");
    assert!("urn:ucode:_0123456789ABCDEF0123456789ABCDEG".parse::<Ucode>().is_err(), "G is not a hex digit");
}

#[test]
fn bytes_and_chunks_round_trip() {
    let code = Ucode::from_u128(0xDEAD_BEEF_0BAD_F00D_1234_5678_9ABC_DEF0);
    let bytes = code.to_bytes();
    assert_eq!(bytes.len(), 16);
    assert_eq!(Ucode::from_bytes(&bytes).unwrap(), code);
    assert!(Ucode::from_bytes(&bytes[..15]).is_err(), "15 bytes is not a chunk");
    assert!(Ucode::from_bytes(&[]).is_err());
}

#[test]
fn four_ways_of_writing_the_same_doi_share_one_canonical_form() {
    let expected = "doi:10.1000/xyz";
    for input in [
        "doi:10.1000/XYZ",
        "https://doi.org/10.1000/XYZ",
        "http://dx.doi.org/10.1000/XYZ",
        "https://www.doi.org/10.1000/xyz",
    ] {
        let ident = Ident::parse(input).unwrap();
        assert_eq!(ident.canonical(), expected, "{input}");
        assert_eq!(ident.url().as_deref(), Some("https://doi.org/10.1000/xyz"));
    }
}

#[test]
fn a_doi_that_is_not_a_doi_is_refused() {
    for input in ["doi:1000/xyz", "doi:10.12/xyz", "doi:10.1000/", "doi:10.1000/a b"] {
        let error = Ident::parse(input).unwrap_err();
        assert!(error.to_string().contains("DOI"), "{input}: {error}");
    }
}

#[test]
fn arxiv_versions_are_kept_and_pdf_links_are_unwrapped() {
    let ident = Ident::parse("https://arxiv.org/pdf/2101.00001v2.pdf").unwrap();
    assert_eq!(ident.canonical(), "arXiv:2101.00001v2");
    assert_eq!(ident.url().as_deref(), Some("https://arxiv.org/abs/2101.00001v2"));

    // No version given: the canonical form does not invent v1.
    let ident = Ident::parse("arXiv:2101.00001").unwrap();
    assert_eq!(ident.canonical(), "arXiv:2101.00001");
    assert!(ident.notes().iter().any(|note| note.contains("version")));

    // The old identifier style.
    let ident = Ident::parse("arXiv:hep-th/9901001").unwrap();
    assert_eq!(ident.canonical(), "arXiv:hep-th/9901001");

    assert!(Ident::parse("arXiv:not-an-id").is_err());
}

#[test]
fn handles_keep_their_prefix_and_suffix() {
    let ident = Ident::parse("https://hdl.handle.net/4263537/1").unwrap();
    assert_eq!(ident.canonical(), "hdl:4263537/1");
    assert_eq!(ident.url().as_deref(), Some("https://hdl.handle.net/4263537/1"));
    assert!(Ident::parse("hdl:abc/1").is_err(), "a handle prefix is numeric");
    assert!(Ident::parse("hdl:4263537").is_err(), "a handle needs a suffix");
}

#[test]
fn isbn_checksums_are_verified_and_ten_digits_become_thirteen() {
    let ident = Ident::parse("isbn:978-0-306-40615-7").unwrap();
    assert_eq!(ident.canonical(), "urn:isbn:9780306406157");

    // ISBN-10 0-306-40615-2 is the same book; the canonical form is the thirteen-digit one.
    let ident = Ident::parse("0-306-40615-2").unwrap();
    assert_eq!(ident.canonical(), "urn:isbn:9780306406157");

    let error = Ident::parse("isbn:9780306406158").unwrap_err();
    assert!(error.to_string().contains("check digit"), "{error}");
    assert!(Ident::parse("isbn:12345").is_err(), "five digits is not an ISBN");
}

#[test]
fn a_cid_is_checked_as_a_cid_not_as_a_string_that_decodes() {
    // bafkrei… is a raw block; bafybei… is a dag-pb node. Same shape, different meaning.
    let raw = Ident::parse("bafkreigh2akiscaildcqabsyg3dfr6chu3fgpregiymsck7e7aqa4s52zy").unwrap();
    assert_eq!(raw.kind(), "IPFS CID");
    assert_eq!(raw.canonical(), "ipfs://bafkreigh2akiscaildcqabsyg3dfr6chu3fgpregiymsck7e7aqa4s52zy");
    match &raw {
        Ident::Cidv1 { codec, .. } => {
            assert_eq!(*codec, 0x55, "raw");
            assert_eq!(codec_name(*codec), "raw");
        }
        other => panic!("expected a CID, got {other:?}"),
    }
    assert!(raw.notes().iter().any(|note| note.contains("raw block")));

    let node = Ident::parse("ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi").unwrap();
    assert!(node.notes().iter().any(|note| note.contains("dag-pb")), "{:?}", node.notes());

    // A base32 string that is not a well-formed CIDv1 must be refused. Two honest notes:
    // a changed digest digit is still a valid CID (a digest carries no checksum of its own), and an
    // extra base32 character can decode to the same bytes because the leftover bits are dropped —
    // that is a property of base32, not something a CID parser can reject. Truncation, a wrong
    // version and a wrong multihash are the cases that are structurally detectable.
    let valid = "bafkreigh2akiscaildcqabsyg3dfr6chu3fgpregiymsck7e7aqa4s52zy";
    assert!(Ident::parse(&valid[..valid.len() - 6]).is_err(), "a truncated digest");
    assert!(Ident::parse("bafk").is_err(), "too short to hold a version, a codec and a digest");
    assert!(Ident::parse("QmYwAPJzv5CZsnAzt8auVZRn5oQ6d8F6z6sV9z1KkKkKkK").is_err(), "CIDv0 is base58, not base32");
}

#[test]
fn a_bare_ucode_is_recognised_and_canonicalised() {
    let ident = Ident::parse("0123456789ABCDEF0123456789ABCDEF").unwrap();
    assert_eq!(ident.kind(), "ucode");
    assert_eq!(ident.canonical(), "urn:ucode:_0123456789ABCDEF0123456789ABCDEF");
    assert_eq!(ident.url(), None, "ucode resolution is a protocol, not a URL we can promise");
}

#[test]
fn something_that_is_none_of_these_says_what_it_expected() {
    let error = Ident::parse("hello world").unwrap_err();
    let text = error.to_string();
    assert!(text.contains("ucode") && text.contains("DOI") && text.contains("CID"), "{text}");
}
