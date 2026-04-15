use quote::quote;

use crate::ast::attr::MainContainer;
use crate::ast::types;

/// Generates the tokens for the entire [`tinyklv::prelude::Encode`](https://docs.rs/tinyklv/latest/tinyklv/prelude/trait.Encode.html) implementation
pub(crate) fn gen_encode_impl(
    input: &MainContainer,
    key_encoder: &types::XcoderType,
    len_encoder: &types::XcoderType,
) -> proc_macro2::TokenStream {
    let name = &input.ident;
    let sentinel = input.attrs.sentinel.as_ref();
    let items_encoded = gen_items_encoded(&input, &key_encoder, &len_encoder);
    let encode_with_key_len = match sentinel {
        Some(sentinel) => quote! {
            #[automatically_derived]
            impl ::tinyklv::traits::Encode<Vec<u8>> for #name {
                fn encode(&self) -> Vec<u8> {
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
        impl ::tinyklv::traits::EncodeValue<Vec<u8>> for #name {
            fn encode_value(&self) -> Vec<u8> {
                let mut output = vec![];
                #items_encoded
                output
            }
        }
        #encode_with_key_len
    }
}

fn gen_items_encoded(
    input: &MainContainer,
    key_encoder: &types::XcoderType,
    len_encoder: &types::XcoderType,
) -> proc_macro2::TokenStream {
    let items_encoded = input.data.iter().filter_map(|field| match &field.attrs {
        Some(attr) => Some((&field.name, attr)),
        None => None
    }).map(|(name, attrs)| {
        #[allow(clippy::unwrap_used)]
        // `gen_encode_impl` call ensures that `attrs.enc` is `Some`
        let value_encoder = attrs.enc.as_ref().unwrap();
        let key = &attrs.key;
        quote! {
            output.extend(#value_encoder(&self.#name).into_klv(#key_encoder(#key), #len_encoder));
        }
    });
    quote! { #(#items_encoded)* }
}
