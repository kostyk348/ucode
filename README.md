# ucode

**ucode as [RFC 6588](https://www.rfc-editor.org/rfc/rfc6588.txt) defines it — a 128-bit number that
carries no meaning of its own — and the layer that puts DOI, arXiv, handle, ISBN and IPFS identifiers
into one canonical form.**

---

**По-русски коротко.** ucode — это идентификатор из проекта TRON (и ITU-T H.642): 128 бит, которые
присваиваются вещи, месту или даже понятию, и **не содержат в себе никакого смысла** — ни поля
производителя, ни версии, ни префикса. Всё, что нужно знать, лежит на сервере, к которому номер
*резолвится*, и именно поэтому ucode переживает смерть организации, которая его выдала. Номер не
переиспользуется никогда. Здесь реализована стандартизованная строка из RFC 6588
(`urn:ucode:_` + 32 и более заглавных hex-цифр) и, сверх неё, приведение DOI/arXiv/handle/ISBN/IPFS
к одному каноническому виду — чтобы «та же самая вещь» имела один ключ, а не шесть.

---

## What a ucode is

- **128 bits**, extensible in 128-bit units — RFC 6588's grammar is `1*ucode-value`, so a 256-bit
  ucode is a valid URN today and a parser that insists on exactly 32 hex digits will break one day.
- **Assigned to objects, places and concepts.** The whole point is that a shelf in an office can have
  its own number, and so can a corporation or a lot.
- **No sub-fields, no meaning.** "ucode is only an identification number, and its value (or its
  sub-bitfields) does not contain any pre-assigned meaning" — RFC 6588, §4. Anything a scheme like
  GTIN or a UUID version field would tell you from the number itself, ucode tells you from a server.
- **Never recycled, issued by anyone.** The Ubiquitous ID Center allocates subspaces; a factory can
  issue on the spot. The binding between number and thing is permanent.
- **Resolved, not parsed.** The Simplified ucode Resolution Protocol (UID-00005) and the ITU-T H.642.x
  recommendations define how you get from a number to information about it.

The string form is the only part standardised in the IETF, and it is small enough to get wrong:

```text
UCODE-URN    = "urn:ucode:" ucode-name
ucode-name   = "_" ucode-number          ; the underscore makes it an XML-safe identifier
ucode-number = 1*ucode-value             ; one or more, because ucode grows in 128-bit units
ucode-value  = 32HEXDIG                  ; uppercase only — the namespace is case-sensitive
```

```
urn:ucode:_0123456789ABCDEF0123456789ABCDEF
```

## Usage

```rust
use ucode::Ucode;

let id: Ucode = "urn:ucode:_0123456789ABCDEF0123456789ABCDEF".parse()?;
assert_eq!(id.chunks(), 1);
assert_eq!(id.as_u128(), Some(0x0123_4567_89AB_CDEF_0123_4567_89AB_CDEF));
assert_eq!(id.to_string(), "urn:ucode:_0123456789ABCDEF0123456789ABCDEF");
# Ok::<(), Box<dyn std::error::Error>>(())
```

```console
$ ucode parse urn:ucode:_0123456789ABCDEF0123456789ABCDEF
urn:ucode:_0123456789ABCDEF0123456789ABCDEF
  kind: ucode

$ ucode parse doi:10.1000/XYZ
doi:10.1000/xyz
  kind: DOI
  url:  https://doi.org/10.1000/xyz
  note: DOIs are case-insensitive; the canonical form is lowercase

$ ucode check ids.txt
OK  doi:10.1000/xyz
OK  urn:ucode:_0123456789ABCDEF0123456789ABCDEF
ERR isbn:9780306406158: not a valid ISBN: "9780306406158" has a wrong ISBN-13 check digit (expected 7)
```

`ucode parse --json` prints one object per identifier, for scripts. `ucode check` reads a file or
stdin, one identifier per line, and exits non-zero if anything is rejected — a linter for the
identifier column of a database.

## Why the other five schemes are in here

A ucode is the only identifier in this crate that promises to survive its issuer. The rest promise
less and are used more, and they all arrive in several shapes:

| scheme | accepted on input | canonical form |
|---|---|---|
| ucode | `urn:ucode:_…`, bare hex, either case | `urn:ucode:_<UPPERCASE HEX>` |
| DOI | `doi:…`, `https://doi.org/…`, `http://dx.doi.org/…` | `doi:10.<registrant>/<suffix>`, lowercase |
| arXiv | `arXiv:…`, `https://arxiv.org/abs/…`, `…/pdf/….pdf` | `arXiv:<id>[vN]` |
| handle | `hdl:…`, `https://hdl.handle.net/…` | `hdl:<prefix>/<suffix>` |
| ISBN | `urn:isbn:`, `isbn:`, bare digits, hyphens | `urn:isbn:<13 digits>` |
| IPFS CIDv1 | `ipfs://…`, `https://ipfs.io/ipfs/…`, bare base32 | `ipfs://<cid>` |

Validation is real, not decorative: DOI registrant rules (four to nine digits), arXiv id shapes (new
`NNNN.NNNNN` and old `archive/YYMMNNN`), handle prefixes, ISBN-10 and ISBN-13 check digits with
ISBN-10 promoted to ISBN-13, and a CID checked as `<version><codec><multihash>` with a sha2-256
digest — because a base32 string that decodes is not automatically a content address, and a dag-pb
node is not the raw bytes it points at.

## What this crate refuses to do

- **It does not issue ucodes.** Allocation belongs to the Ubiquitous ID Center; a tool that minted
  numbers would be creating identifiers nobody can resolve.
- **It does not resolve them.** Resolution is a protocol (UID-00005, ITU-T H.642.3) and a set of
  servers. `Ident::url()` returns `None` for a ucode on purpose: inventing `https://ucode.example/…`
  would be a promise this crate cannot keep. Every other scheme gets the resolver that really exists.
- **It does not guess a tag layout.** The binary carrier is the ucode Tag Architecture (UID-00017);
  `Ucode::to_bytes` is the plain 128-bit value and says so.
- **It does not claim a digest is verifiable.** A changed CID digit yields another structurally valid
  CID: digests have no checksum, and only structure can be checked here.

## Tests

```sh
cargo test
```

Thirteen tests plus a doctest, no dependencies, no network:

- the RFC 6588 example is the canonical form, and `to_string` reproduces it exactly;
- a 256-bit ucode (two chunks, 64 hex digits) parses, reports two chunks and explains itself;
- lowercase hex is refused by the strict door and normalised by the lenient one;
- the underscore is mandatory, chunk lengths are byte-exact, and `G` is not a hex digit;
- bytes and chunks round-trip, and 15 bytes is not a chunk;
- four spellings of one DOI collapse to one key, and a malformed DOI is refused with the rule it broke;
- arXiv versions are kept, `.pdf` links are unwrapped, and no version is invented;
- handles keep prefix and suffix, and a non-numeric prefix is refused;
- ISBN-10 `0-306-40615-2` and ISBN-13 `978-0-306-40615-7` produce the same canonical key, and a wrong
  check digit is caught;
- a raw CIDv1 and a dag-pb CIDv1 are distinguished by codec, a truncated digest is refused, and CIDv0
  is rejected as "base58, not base32";
- an unrecognised string names every scheme it knows.

## Sources

- **RFC 6588**, C. Ishikawa, "A URN Namespace for ucode", April 2012 — the grammar, the case rule,
  the `1*` extensibility, the resolution pointers. (IANA URN namespace NID 46.)
- **ITU-T F.771** (application requirements), **H.621** (system architecture), **H.642.1/.2/.3** (ID
  system, registration, interpretation protocol).
- **uidcenter.org**: UID-00010 (`ucode: Ubiquitous Code`), UID-00005 (resolution protocol),
  UID-00017 (tag architecture), UID-00034 (management procedures).
- `TRONSHOW 2014, uID Architecture 2.0` (K. Sakamura) — 150 million-identifier space, applications
  (house registration, horse pedigrees, drug traceability, GSI control points).

## Licence

MIT OR Apache-2.0.
