mod decode_impl;
mod encode_impl;
pub(crate) mod helpers;

use proc_macro2::TokenStream;
use quote::quote;

use crate::ast::attr::MainContainer;
use crate::Ctxt;

pub fn derive(input: &syn::DeriveInput) -> syn::Result<TokenStream> {
    // --------------------------------------------------
    // create a new error context
    // --------------------------------------------------
    let cx = Ctxt::new();
    // --------------------------------------------------
    // get the parsed container for Klv derive
    // --------------------------------------------------
    let cont = match MainContainer::from_ast(&cx, input) {
        Some(cont) => cont,
        None => return Err(cx.check().unwrap_err()),
    };
    // --------------------------------------------------
    // check for errors
    // --------------------------------------------------
    cx.check()?;
    // --------------------------------------------------
    // check to see if all encoders/decoders exist
    // --------------------------------------------------
    let mut all_encoders_exist = true;
    let mut all_decoders_exist = true;
    let mut any_encoders_exist = false;
    let mut any_decoders_exist = false;
    for f in cont.data.iter().filter_map(|f| f.attrs.as_ref()) {
        any_encoders_exist |= f.enc.is_some();
        any_decoders_exist |= f.dec.is_some();
        all_encoders_exist &= f.enc.is_some();
        all_decoders_exist &= f.dec.is_some();
    }
    all_encoders_exist &= any_encoders_exist;
    all_decoders_exist &= any_decoders_exist;
    // --------------------------------------------------
    // init
    // --------------------------------------------------
    let mut expanded = quote! {};
    // --------------------------------------------------
    // impl decode
    // --------------------------------------------------
    if let (Some(key_dec), Some(len_dec), true) = (
        cont.attrs.key.dec.as_ref(),
        cont.attrs.len.dec.as_ref(),
        all_decoders_exist,
    ) {
        let decode_impls = decode_impl::gen_decode_impl(&cont, key_dec, len_dec);
        expanded = quote! {
            #expanded
            #decode_impls
        }
    }
    // --------------------------------------------------
    // impl encode
    // --------------------------------------------------
    if let (Some(key_enc), Some(len_enc), true) = (
        cont.attrs.key.enc.as_ref(),
        cont.attrs.len.enc.as_ref(),
        all_encoders_exist,
    ) {
        let encode_impls = encode_impl::gen_encode_impl(&cont, &key_enc, &len_enc);
        expanded = quote! {
            #expanded
            #encode_impls
        }
    }

    // --------------------------------------------------
    // testing tokens -> fn, for doc commenting, something to consider?
    // --------------------------------------------------
    // let name = cont.ident;
    // let aue = cont.attrs.aue_path.unwrap_or(symbol::ALLOW_UNIMPLEMENTED_ENCODE.into());
    // expanded = quote! {
    //     #expanded
    //     impl ::tinyklv::prelude::TinyklvDoc for #name {
    //         fn #aue() { }
    //     }
    // };

    Ok(expanded)
}
