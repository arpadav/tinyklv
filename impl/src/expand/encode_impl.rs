//! Token generation for the [`tinyklv::prelude::Encode`](https://docs.rs/tinyklv/latest/tinyklv/prelude/trait.Encode.html)
//! and [`tinyklv::prelude::EncodeValue`](https://docs.rs/tinyklv/latest/tinyklv/prelude/trait.EncodeValue.html)
//! derive implementations
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::ast::attr::MainContainer;
use crate::ast::types;

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::quote;

/// Generates the tokens for the entire [`tinyklv::prelude::Encode`](https://docs.rs/tinyklv/latest/tinyklv/prelude/trait.Encode.html)
/// and [`tinyklv::prelude::EncodeValue`](https://docs.rs/tinyklv/latest/tinyklv/prelude/trait.EncodeValue.html)
/// implementations for a container
///
/// Emits an `EncodeValue` impl for every container. If a sentinel is present,
/// also emits a full `Encode` impl that wraps the value encoding in a KLV
/// packet (key + length + value)
///
/// # Arguments
///
/// * `input` - The parsed container descriptor including field metadata
/// * `key_encoder` - Token expression used to encode each field's key
/// * `len_encoder` - Token expression used to encode the packet length
pub(crate) fn gen_encode_impl(
    input: &MainContainer,
    key_encoder: &types::XcoderType,
    len_encoder: &types::XcoderType,
) -> proc_macro2::TokenStream {
    // --------------------------------------------------
    // extract the container name and sentinel, if any
    // --------------------------------------------------
    let name = &input.ident;
    let sentinel = input.attrs.sentinel.as_ref();
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    // --------------------------------------------------
    // generate the per-field encoding token stream
    // --------------------------------------------------
    let items_encoded = gen_items_encoded(input, key_encoder, len_encoder);
    // --------------------------------------------------
    // if a sentinel is present, also emit the full
    // `EncodeFrame` impl that wraps the value in a KLV packet
    // --------------------------------------------------
    let encode_with_key_len = match sentinel {
        Some(sentinel) => quote! {
            #[automatically_derived]
            impl #impl_generics ::tinyklv::traits::EncodeFrame<Vec<u8>> for #name #ty_generics #where_clause {
                fn encode_frame(&self) -> Vec<u8> {
                    self.encode_value().into_klv(
                        #sentinel,
                        #len_encoder ,
                    )
                }
            }
        },
        None => quote! {},
    };
    quote! {
        #[automatically_derived]
        impl #impl_generics ::tinyklv::traits::EncodeValue<Vec<u8>> for #name #ty_generics #where_clause {
            fn encode_value(&self) -> Vec<u8> {
                let mut output = vec![];
                #items_encoded
                output
            }
        }
        #encode_with_key_len
    }
}

/// Generates the token stream that encodes each field of a container into
/// the output byte buffer
///
/// Iterates over fields that have a `#[klv(..)]` attribute and emits one
/// `output.extend(...)` expression per field. Optional fields are wrapped
/// in an `if let Some(ref __val)` guard so they are skipped when absent
///
/// # Arguments
///
/// * `input` - The parsed container descriptor including field metadata
/// * `key_encoder` - Token expression used to encode each field's key
/// * `len_encoder` - Token expression used to encode the value length
///
/// # Safety
///
/// Uses `.unwrap()` on `attrs.enc` - this is safe because `gen_encode_impl`
/// only calls this function when `all_encoders_exist` is `true`, guaranteeing
/// every field attribute carries a non-`None` encoder
fn gen_items_encoded(
    input: &MainContainer,
    key_encoder: &types::XcoderType,
    len_encoder: &types::XcoderType,
) -> proc_macro2::TokenStream {
    // --------------------------------------------------
    // map each attributed field to its encoding expression
    // --------------------------------------------------
    let items_encoded = input
        .data
        .iter()
        .filter_map(|field| {
            field
                .attrs
                .as_ref()
                .map(|attr| (&field.name, &field.ty, attr))
        })
        .map(|(name, ty, attrs)| {
            #[allow(
                clippy::unwrap_used,
                reason = "`gen_encode_impl` call ensures that `attrs.enc` is `Some`"
            )]
            let value_encoder = attrs.enc.as_ref().unwrap();
            let key = &attrs.key;
            // --------------------------------------------------
            // optional fields are skipped when absent
            // --------------------------------------------------
            if crate::expand::helpers::is_option(ty) {
                quote! {
                    if let Some(ref __val) = self.#name {
                        output.extend(#value_encoder(__val).into_klv(#key_encoder(#key), #len_encoder));
                    }
                }
            } else {
                quote! {
                    output.extend(#value_encoder(&self.#name).into_klv(#key_encoder(#key), #len_encoder));
                }
            }
        });
    quote! { #(#items_encoded)* }
}
