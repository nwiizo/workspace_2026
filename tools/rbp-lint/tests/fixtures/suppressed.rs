// rbp-lint-allow: no-panic (file-top: this fixture intentionally panics)

pub fn raw_unwrap() -> String {
    // rbp-lint-allow: no-unwrap (preceding comment scope)
    std::fs::read_to_string("a").unwrap()
}

pub fn raw_unwrap_inline() -> String {
    std::fs::read_to_string("b").unwrap() // rbp-lint-allow: no-unwrap
}

pub fn still_panics() {
    panic!("file-top allow keeps no-panic silent here");
}

// no suppression for this one — should still fire
pub fn unsuppressed_unwrap() -> String {
    std::fs::read_to_string("c").unwrap()
}
