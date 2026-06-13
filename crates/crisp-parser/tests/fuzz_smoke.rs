//! Fuzz smoke: lexer and parser must not panic on arbitrary input.

use crisp_lexer::lex;
use crisp_parser::Parser;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

fn seeded_string(seed: u64, len: usize) -> String {
    let mut out = String::with_capacity(len);
    let mut state = seed;
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz_{}():=<>!&|\"'\n\t01";
    for _ in 0..len {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let idx = (state >> 33) as usize % CHARSET.len();
        out.push(CHARSET[idx] as char);
    }
    out
}

fn hash_string(s: &str) -> u64 {
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

#[test]
fn fuzz_lexer_never_panics() {
    for i in 0..500u64 {
        let s = seeded_string(i, (i as usize % 80) + 1);
        eprintln!("lex seed={i} len={}", s.len());
        let _ = lex(&s);
    }
}

#[test]
fn fuzz_parser_never_panics() {
    for i in 0..300u64 {
        let s = seeded_string(i.wrapping_mul(97), (i as usize % 120) + 1);
        eprintln!("parse seed={i} hash={}", hash_string(&s));
        if let Ok(mut p) = Parser::new(&s) {
            let _ = p.parse_module();
        }
    }
}

#[test]
fn fuzz_parse_valid_fragments() {
    let fragments = [
        "pub main() = print(\"hi\")",
        "greet(x) = x",
        "type T = { a: int }",
        "extern \"C\" { abs(x: int) -> int }",
        "test \"t\" = { assert_eq(1, 1) }",
        "f() = match 1 { x -> \"ok\" }",
        "f() = if true then 1 else 0",
        "f() = unsafe { abs(1) }",
    ];
    for (i, frag) in fragments.iter().enumerate() {
        eprintln!("fragment {i}: {frag}");
        let mut p = Parser::new(frag).expect("parser");
        p.parse_module().expect("parse");
    }
}
