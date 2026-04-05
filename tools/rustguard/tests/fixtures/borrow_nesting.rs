/// Borrow nesting pattern tests (for Phase 2).

// Excessive borrow nesting: &&T where &T would suffice
pub fn excessive_ref(x: &&i32) -> i32 {
    **x
}

// Triple reference nesting
pub fn triple_ref(x: &&&i32) -> i32 {
    ***x
}

// Normal single reference (should not trigger)
pub fn normal_ref(x: &i32) -> i32 {
    *x
}

// Mutable reference to reference
pub fn mut_ref_to_ref(x: &mut &i32) -> i32 {
    **x
}
