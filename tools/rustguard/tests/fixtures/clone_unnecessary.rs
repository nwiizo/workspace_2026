/// Clone pattern tests (for Phase 2).

// Unnecessary clone: s is not used after cloning
pub fn unnecessary_clone(s: String) -> String {
    let cloned = s.clone();
    cloned
}

// Necessary clone: both s and cloned are used
pub fn necessary_clone(s: String) -> (String, String) {
    let cloned = s.clone();
    (s, cloned)
}

// Clone in a loop
pub fn clone_in_loop(items: &[String]) -> Vec<String> {
    let mut result = Vec::new();
    for item in items {
        result.push(item.clone());
    }
    result
}
