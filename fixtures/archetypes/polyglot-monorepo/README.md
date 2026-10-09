# Polyglot Monorepo archetype

## Purpose

Deterministic eval fixture (evallerina-sqq, folded from the Qwen benchmark
proposal): a three-language monorepo stressing cross-tool workflows of the
dulce-de-leche suite, with one injected fault per the archetype contract.

## Contents

- `rust/ffi.rs` — the Rust FFI surface (`scale_value` extern + internal
  `ffi_scale` helper).
- `python/binding.py` — Python binding over the same seam; **injected
  fault**: `scale_value` declares a `str` codomain while its callee
  produces `int` (vampiro REQ-7 return-boundary composition break).
- `ts/client.ts` — TypeScript caller for the wasm build. vampiro 0.5.0
  does **not** scan `.ts` in directory mode (silently skipped: 2 of 3
  files scanned) despite the tool README advertising TS support — a
  self-reported-vs-actual delta this archetype records, never hard-asserts.

## Replay shape

`vampiro check --full -p . --mode gate -j` blocks (exit 3) on the REQ-7
finding; `--mode guidance -j` reports the same seam advisory (exit 0).
The smoke scenario replays this gate→guidance path
(`scenarios/archetypes/vampiro-polyglot-seam.json`).

## Rationale

Composition-seam faults are exactly what vampiro exists to catch, and the
Rust/Python pair exercises the cross-language surface without depending on
a live wasm build. Determinism: no timestamps, no network, no tool state —
every file is byte-stable, so captures replay identically offline.