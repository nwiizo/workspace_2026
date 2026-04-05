// Test fixture: struct layout patterns for RustLean analysis

// Struct with potential padding waste (bool, u64, bool = 24 bytes instead of 10)
struct PaddedStruct {
    a: bool,    // 1 byte
    b: u64,     // 8 bytes
    c: bool,    // 1 byte
}

// Large struct that should trigger a warning when moved
struct LargeStruct {
    data: [u8; 256],
    name: String,
    values: Vec<f64>,
}

fn use_padded(p: PaddedStruct) -> bool {
    p.a && p.c
}

fn move_large_struct() -> LargeStruct {
    let s = LargeStruct {
        data: [0u8; 256],
        name: String::from("test"),
        values: vec![1.0, 2.0, 3.0],
    };
    s // large struct moved
}

fn main() {
    let p = PaddedStruct { a: true, b: 42, c: false };
    let _ = use_padded(p);
    let _ = move_large_struct();
}
