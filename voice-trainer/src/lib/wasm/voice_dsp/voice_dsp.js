// Stub — overwritten by `npm run wasm` once the Rust crate is built.
// While this file is present, the app uses the JS pitch-detection fallback in wasm.ts.

export function detect_pitch(_samples, _sample_rate) { return 0; }
export default async function init() { throw new Error('WASM not built — run `npm run wasm`'); }
