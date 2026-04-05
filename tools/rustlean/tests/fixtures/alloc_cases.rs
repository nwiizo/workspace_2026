// Test fixture: allocation patterns for RustLean analysis

fn box_allocation() -> Box<i32> {
    Box::new(42)
}

fn vec_allocation() -> Vec<i32> {
    let mut v = Vec::new();
    v.push(1);
    v.push(2);
    v.push(3);
    v
}

fn string_allocation() -> String {
    String::from("hello world")
}

fn format_allocation(name: &str) -> String {
    format!("hello, {name}")
}

fn allocation_in_loop(n: usize) -> Vec<String> {
    let mut result = Vec::new();
    for i in 0..n {
        let s = format!("item {i}"); // allocation in loop
        result.push(s);
    }
    result
}

fn main() {
    let _ = box_allocation();
    let _ = vec_allocation();
    let _ = string_allocation();
    let _ = format_allocation("world");
    let _ = allocation_in_loop(10);
}
