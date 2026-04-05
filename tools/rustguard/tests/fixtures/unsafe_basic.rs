/// Basic unsafe patterns for testing RustGuard detection.

// An unsafe function — should be detected as RG001
pub unsafe fn raw_pointer_deref(ptr: *const i32) -> i32 {
    *ptr
}

// A function containing an unsafe block without SAFETY comment — RG002 + suggestion
pub fn safe_wrapper(value: &i32) -> i32 {
    let ptr: *const i32 = value;
    unsafe { *ptr }
}

// A function with a proper SAFETY comment — RG002 but no suggestion
pub fn safe_wrapper_with_comment(value: &i32) -> i32 {
    let ptr: *const i32 = value;
    // SAFETY: ptr is derived from a valid reference and used immediately
    unsafe { *ptr }
}

// A function that calls the unsafe function — should appear in unsafe reach (RG003)
pub fn caller_of_unsafe() -> i32 {
    let x = 42;
    // SAFETY: we pass a valid pointer
    unsafe { raw_pointer_deref(&x) }
}

// A function that transitively depends on unsafe through safe_wrapper
pub fn transitive_caller() -> i32 {
    safe_wrapper(&10) + caller_of_unsafe()
}

// Multiple unsafe blocks in one function
pub fn multiple_unsafe_blocks() -> (i32, i32) {
    let a = 1i32;
    let b = 2i32;
    let pa: *const i32 = &a;
    let pb: *const i32 = &b;
    // SAFETY: both pointers are derived from valid references
    let va = unsafe { *pa };
    let vb = unsafe { *pb };
    (va, vb)
}
