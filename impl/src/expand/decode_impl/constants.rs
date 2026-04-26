// --------------------------------------------------
// external
// --------------------------------------------------
use proc_macro2::TokenStream;
use quote::quote;

/// debug logger used in generated code
pub(super) fn logger() -> proc_macro2::TokenStream {
    #[cfg(feature = "tracing")]
    {
        quote! { ::tracing::debug! }
    }
    #[cfg(not(feature = "tracing"))]
    {
        quote! { ::std::println! }
    }
}

/// Returns the ident of the generated partial-packet struct for a given
/// container ident, e.g. `Xxx` -> `XxxPartialPacket`. The naming is
/// load-bearing for the `[`tinyklv::traits::DecodePartial::Partial`]
/// associated type emitted by [`gen_decode_impl`].
pub(super) fn create_partial_name(name: &syn::Ident) -> syn::Ident {
    quote::format_ident!("{}PartialPacket", name)
}

/// packet lifetime character used in generated struct names
pub(super) fn create_lifetime() -> TokenStream {
    quote! { 'z }
}
