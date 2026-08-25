//! Apeireth host infrastructure facade (v1 era — reconstructed module layout).
//!
//! Original v1 lib.rs also declared `machine_id` and `organ_kani_proofs` modules,
//! but those are out-of-scope for the v2 transcription task. This file is the
//! transcribed v1 lib.rs (verbatim of the file content, with reconstruction
//! notes for the out-of-scope modules).
//!
//! In the v2 layout, this file lives as a sub-module of `v1host`, so it does
//! NOT re-declare `atomic_write`/`three_way`/`keyring` (the parent `mod.rs`
//! handles that). The re-exports are also lifted to `mod.rs`.
//!
//! Hard constraints (carried from v1):
//! - OS keyring preferred (Windows Credential Manager / macOS Keychain / Linux Secret Service)
//! - Encrypted fallback MUST be used when OS keyring is unavailable (no plaintext on disk)
//! - Three-way conflict detection before any destructive operation

#![warn(missing_docs)]

// (No module declarations here — parent v1host/mod.rs handles atomic_write/three_way/keyring.
//  Re-exports are also in parent mod.rs.)
