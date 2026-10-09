//! Purpose: Minimal FFI surface exposed to foreign callers (archetype
//! fixture for evallerina-sqq).
//! Responsibilities: define the Rust side of the cross-language seam —
//! a stable `extern "C"` entry point delegating to an internal helper.
//! Rationale: the composition fault lives on the Python side (declared
//! codomain vs callee codomain); this file is the honest callee.

/// Foreign entry point: scale `x` by `factor`.
#[no_mangle]
pub extern "C" fn scale_value(x: i32, factor: i32) -> i32 {
    ffi_scale(x, factor)
}

/// Internal helper carrying the true arithmetic (codomain i32).
fn ffi_scale(x: i32, factor: i32) -> i32 {
    x.wrapping_mul(factor)
}