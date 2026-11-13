//! `ucode` — identify an identifier, or print the canonical form of a ucode.
//!
//! ```text
//! ucode parse doi:10.1000/XYZ              # what is it, and one canonical key for it
//! ucode parse urn:ucode:_0123...ABCD       # the same for a ucode
//! ucode urn 0123456789ABCDEF0123456789ABCDEF
//! ucode check ids.txt                      # one identifier per line; exit 1 if any is invalid
//! ucode parse --json doi:10.1000/xyz
//! ```

use std::io::{BufRead, IsTerminal};

use ucode::ident::Ident;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
        usage();
        return;
    }
    let json = args.iter().any(|a| a == "--json");
    let positional: Vec<&str> = args.iter().filter(|a| !a.starts_with("--")).map(String::as_str).collect();
    let code = match positional.as_slice() {
        ["parse", rest @ ..] => parse(rest, json),
        ["urn", rest @ ..] => urn(rest, json),
        ["check", rest @ ..] => check(rest, json),
        [other, ..] => {
            eprintln!("ucode: unknown command {other:?}; try --help");
            2
        }
        [] => {
            usage();
            2
        }
    };
    std::process::exit(code);
}

fn usage() {
    println!(
        "ucode — ucode (RFC 6588) and the identifiers that are less honest about what they are\n\n\
           ucode parse <id>...     recognise an identifier, print kind, canonical form, URL and notes\n\
           ucode urn <hex|urn>...  print the canonical urn:ucode: form\n\
           ucode check [file]      validate one identifier per line (stdin if no file)\n\
         options: --json\n\n\
         known: ucode, DOI, arXiv, handle, ISBN, IPFS CIDv1"
    );
}

fn parse(items: &[&str], json: bool) -> i32 {
    if items.is_empty() {
        eprintln!("ucode parse: give at least one identifier");
        return 2;
    }
    let mut failures = 0;
    for item in items {
        match Ident::parse(item) {
            Ok(ident) => {
                if json {
                    println!(
                        "{{\"input\":{},\"kind\":{},\"canonical\":{},\"url\":{},\"notes\":[{}]}}",
                        quote(item),
                        quote(ident.kind()),
                        quote(&ident.canonical()),
                        ident.url().map(|url| quote(&url)).unwrap_or_else(|| "null".to_string()),
                        ident.notes().iter().map(|note| quote(note)).collect::<Vec<_>>().join(",")
                    );
                } else {
                    println!("{}", ident.canonical());
                    eprintln!("  kind: {}", ident.kind());
                    if let Some(url) = ident.url() {
                        eprintln!("  url:  {url}");
                    }
                    for note in ident.notes() {
                        eprintln!("  note: {note}");
                    }
                }
            }
            Err(error) => {
                eprintln!("ucode: {error}");
                failures += 1;
            }
        }
    }
    if failures > 0 { 1 } else { 0 }
}

fn urn(items: &[&str], json: bool) -> i32 {
    if items.is_empty() {
        eprintln!("ucode urn: give at least one ucode");
        return 2;
    }
    let mut failures = 0;
    for item in items {
        match ucode::Ucode::from_hex(item) {
            Ok(code) => {
                let urn = code.to_urn();
                if json {
                    println!("{{\"canonical\":{},\"chunks\":{}}}", quote(&urn), code.chunks());
                } else {
                    println!("{urn}");
                }
            }
            Err(error) => {
                eprintln!("ucode: {item:?}: {error}");
                failures += 1;
            }
        }
    }
    if failures > 0 { 1 } else { 0 }
}

fn check(items: &[&str], json: bool) -> i32 {
    let lines: Vec<String> = match items.first() {
        Some(path) => match std::fs::read_to_string(path) {
            Ok(text) => text.lines().map(str::to_string).collect(),
            Err(error) => {
                eprintln!("ucode: {path}: {error}");
                return 2;
            }
        },
        None => {
            if std::io::stdin().is_terminal() {
                eprintln!("ucode check: give a file or pipe identifiers on stdin");
                return 2;
            }
            std::io::stdin().lock().lines().map_while(Result::ok).collect()
        }
    };
    let mut failures = 0;
    let mut checked = 0;
    for line in lines {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        checked += 1;
        match Ident::parse(line) {
            Ok(ident) => {
                if json {
                    println!("{{\"input\":{},\"ok\":true,\"canonical\":{}}}", quote(line), quote(&ident.canonical()));
                } else {
                    println!("OK  {}", ident.canonical());
                }
            }
            Err(error) => {
                failures += 1;
                if json {
                    println!("{{\"input\":{},\"ok\":false,\"error\":{}}}", quote(line), quote(&error.to_string()));
                } else {
                    println!("ERR {line}: {error}");
                }
            }
        }
    }
    eprintln!("ucode check: {checked} checked, {failures} rejected");
    if failures > 0 { 1 } else { 0 }
}

/// Enough JSON escaping for the strings an identifier can contain.
fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
