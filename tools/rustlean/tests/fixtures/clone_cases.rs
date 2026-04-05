// Test fixture: clone patterns for RustLean analysis

fn unnecessary_clone(s: String) -> String {
    let cloned = s.clone(); // s is not used after this - clone is unnecessary
    cloned
}

fn necessary_clone(s: &String) -> String {
    let cloned = s.clone(); // s is borrowed, clone is needed
    cloned
}

fn clone_in_loop(items: &[String]) -> Vec<String> {
    let mut result = Vec::new();
    for item in items {
        result.push(item.clone()); // clone in loop
    }
    result
}

fn main() {
    let s = String::from("hello");
    let _ = unnecessary_clone(s);

    let s2 = String::from("world");
    let _ = necessary_clone(&s2);

    let items = vec![String::from("a"), String::from("b")];
    let _ = clone_in_loop(&items);
}
