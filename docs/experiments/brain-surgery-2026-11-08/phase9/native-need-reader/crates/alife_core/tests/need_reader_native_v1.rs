//! Detached CPU primitive qualification. No production module or runtime attachment.
// Qualified store uses crate-root imports; these are the unchanged core types.
use alife_core::*;

#[path = "need_reader_native_v1/native.rs"]
mod native;
#[path = "need_reader_native_v1/qualification.rs"]
mod qualification;
#[allow(dead_code)] // Frozen store API is intentionally broader than this bridge.
#[path = "need_reader_native_v1/qualified_store.rs"]
mod qualified_store;
#[path = "need_reader_native_v1/scalar_reference.rs"]
mod scalar_reference;
#[path = "need_reader_native_v1/sealed_fixture.rs"]
mod sealed_fixture;
