// src-tauri/build.rs
//
// Standard Tauri 2.x build script. Generates the type-safe IPC + capability
// schema bindings that `tauri::generate_context!` consumes at compile time.
//
// Phase 1 deliverable. The Phase 1 coder may not need to edit this — the
// scaffold output is identical to this file. Pre-staged so the dependency
// on `tauri-build` is visible in the workspace early.

fn main() {
    tauri_build::build()
}
