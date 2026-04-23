// --------------------------------------------------
// local
// --------------------------------------------------
use crate::ast::types;

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::quote;

/// Code generation for seek sentinel implementation
pub(super) fn gen_sentinel_impl(
    name: &syn::Ident,
    sentinel: &syn::Lit,
    stream: &syn::Type,
    stream_lifetimed: &syn::Type,
    lifetime: &proc_macro2::TokenStream,
    len_decoder: &types::XcoderType,
    generics: &syn::Generics,
) -> proc_macro2::TokenStream {
    // --------------------------------------------------
    // generics
    // --------------------------------------------------
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // --------------------------------------------------
    // seeker using memchr
    // --------------------------------------------------
    let sentinel_len_static_name =
        quote::format_ident!("__TINYKLV_SENTINEL_LEN_{}", name.to_string().to_uppercase());
    let sentinel_seeker_static_name =
        quote::format_ident!("__TINYKLV_SEEKER_{}", name.to_string().to_uppercase());

    // --------------------------------------------------
    // return seek sentinel impl
    // --------------------------------------------------
    quote! {
        #[automatically_derived]
        #[doc(hidden)]
        #[doc = concat!(" Static sentinel length for [`", stringify!(#name), "`] implementation of [`tinyklv::prelude::SeekSentinel`]")]
        pub(crate) const #sentinel_len_static_name: usize = #sentinel.len();

        #[automatically_derived]
        #[doc(hidden)]
        #[doc = concat!(" Static seeker for [`", stringify!(#name), "`] implementation of [`tinyklv::prelude::SeekSentinel`]")]
        pub(crate) static #sentinel_seeker_static_name: ::std::sync::LazyLock<::tinyklv::__export::memchr::memmem::Finder> =
            ::std::sync::LazyLock::new(|| ::tinyklv::__export::memchr::memmem::Finder::new(#sentinel));

        #[automatically_derived]
        #[doc(hidden)]
        #[doc = concat!(" [`", stringify!(#name), "`] implementation of [`tinyklv::prelude::SeekSentinel`] for [`", stringify!(#stream), "`]")]
        impl #impl_generics ::tinyklv::traits::SeekSentinel<#stream> for #name #ty_generics #where_clause {
            fn seek_sentinel<#lifetime>(input: &mut #stream_lifetimed) -> ::tinyklv::__export::winnow::Result<#stream_lifetimed> {
                let checkpoint = input.checkpoint();
                match #sentinel_seeker_static_name.find(&input) {
                    Some(position) => *input = &input[position + #sentinel_len_static_name..],
                    None => return Err(
                        ::tinyklv::__export::winnow::error::ContextError::new().add_context(
                            input,
                            &checkpoint,
                            ::tinyklv::__export::winnow::error::StrContext::Label(
                                concat!("Unable to find recognition sentinel for `", stringify!(#name), "` packet")
                            ),
                        )
                    )
                };
                let checkpoint = input.checkpoint();
                let packet_len = match #len_decoder.parse_next(input) {
                    Ok(x) => x as usize,
                    Err(e) => return Err(
                        e.add_context(
                            input,
                            &checkpoint,
                            ::tinyklv::__export::winnow::error::StrContext::Label(
                                concat!("Unable to parse packet length for `", stringify!(#name), "` packet")
                            ),
                        )
                    ),
                };
                ::tinyklv::__export::winnow::token::take(packet_len).parse_next(input)
            }
        }
    }
}
