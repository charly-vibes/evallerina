"""Purpose: Python binding over the Rust FFI surface (archetype fixture
for evallerina-sqq).
Responsibilities: expose the foreign seam to Python callers; carries the
injected composition fault under eval.
Rationale: `scale_value` declares a `str` codomain while `ffi_scale`
produces `int` — vampiro REQ-7 return-boundary break (medium severity).
"""


def ffi_scale(x: int, factor: int) -> int:
    """Thin wrapper over the Rust extern; true codomain int."""
    return x * factor


def scale_value(x: int, factor: int) -> str:
    """Injected fault: declared str, callee produces int — composition break."""
    return ffi_scale(x, factor)