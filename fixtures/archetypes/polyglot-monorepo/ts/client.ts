// Purpose: TypeScript caller for the wasm build of the same FFI surface
// (archetype fixture for evallerina-sqq).
// Responsibilities: represent the third language of the polyglot triad.
// Rationale: vampiro 0.5.0 silently skips `.ts` files in directory scans
// (README claims TS support) — the fixture keeps this file so captures
// record the 2-of-3 scanned delta honestly.

export function scaleClient(x: number, factor: number): string {
  return String(scaleWasm(x, factor));
}

function scaleWasm(x: number, factor: number): number {
  return x * factor;
}