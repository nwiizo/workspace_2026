/// FFI-related unsafe patterns for testing.

extern "C" {
    fn strlen(s: *const std::os::raw::c_char) -> usize;
}

// FFI call wrapped in unsafe — should detect unsafe block (RG002)
pub fn get_c_string_len(s: &std::ffi::CStr) -> usize {
    // SAFETY: CStr guarantees a valid null-terminated C string
    unsafe { strlen(s.as_ptr()) }
}

// Unsafe block with raw pointer arithmetic
pub fn offset_pointer(data: &[u8], offset: usize) -> Option<u8> {
    if offset >= data.len() {
        return None;
    }
    let ptr = data.as_ptr();
    // SAFETY: offset is bounds-checked above
    Some(unsafe { *ptr.add(offset) })
}

// Unsafe trait implementation
pub struct MyType;

unsafe impl Send for MyType {}
unsafe impl Sync for MyType {}
