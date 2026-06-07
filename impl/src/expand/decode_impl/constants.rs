//! Shared constants and naming helpers for the decode code-generation pass
//!
//! Provides small utilities that are reused across the decode codegen
//! sub-modules: the partial-packet struct naming convention and the lifetime
//! token used in generated stream references. The logging-macro selection lives
//! in [`crate::expand::logger`], shared with the encode pass
//!
//! Author: aav
// --------------------------------------------------
// external
// --------------------------------------------------
use proc_macro2::TokenStream;
use quote::quote;

/// Derives the partial-packet struct ident from a container ident
///
/// Appends `PartialPacket` to the container name, producing e.g
/// `MyPacket` -> `MyPacketPartialPacket`. This naming is load-bearing:
/// the [`tinyklv::traits::DecodePartial::Partial`] associated type emitted
/// by the decode codegen refers to this exact ident
///
/// # Arguments
///
/// * `name` - The container ident to derive the partial name from
///
/// # Returns
///
/// A new [`syn::Ident`] with `PartialPacket` appended to `name`
pub(super) fn create_partial_name(name: &syn::Ident) -> syn::Ident {
    quote::format_ident!("{}PartialPacket", name)
}

/// Returns the lifetime token used in generated stream-reference types
///
/// The fixed lifetime `'z` is used consistently across the decode codegen
/// so that generated `impl` blocks and function signatures all refer to
/// the same lifetime parameter without risk of collision with user lifetimes
///
/// # Returns
///
/// A [`proc_macro2::TokenStream`] containing the `'z` lifetime token
pub(super) fn create_lifetime() -> TokenStream {
    quote! { 'z }
}
