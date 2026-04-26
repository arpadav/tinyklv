// --------------------------------------------------
// mods
// --------------------------------------------------
mod constants;
mod decode_partial_gen;
mod key_match_gen;
mod partial_gen;
mod sentinel_gen;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::{
    ast::{attr::MainContainer, types},
    expand::helpers,
};

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::quote;

/// Generates the tokens for the entire [`tinyklv::prelude::DecodeValue`]
/// and associated decode traits implementation
pub(crate) fn gen_decode_impl(
    input: &MainContainer,
    key_decoder: &types::XcoderType,
    len_decoder: &types::XcoderType,
) -> proc_macro2::TokenStream {
    // --------------------------------------------------
    // get name and partial name
    // --------------------------------------------------
    let name = &input.ident;
    let partial_name = constants::create_partial_name(name);

    // --------------------------------------------------
    // default stream -> &[u8]
    // --------------------------------------------------
    let stream = input.attrs.stream.clone().unwrap_or(helpers::u8_slice());
    let lifetime = constants::create_lifetime();
    let stream_lifetimed = helpers::insert_lifetime(&stream, lifetime.clone());

    // --------------------------------------------------
    // seek implementation
    // --------------------------------------------------
    let sentinel = input.attrs.sentinel.as_ref();
    let seek_if_sentinel = match sentinel {
        Some(sentinel) => sentinel_gen::gen_sentinel_impl(
            name,
            sentinel,
            &stream,
            &stream_lifetimed,
            &lifetime,
            len_decoder,
            input.generics,
        ),
        None => quote! {},
    };

    // --------------------------------------------------
    // * the init sequence at beginning of decode, partial struct definition
    // * impl Partial for partial struct, which converts
    //   the partial packet -> final struct
    // * impl TryFrom<partial struct> for final struct
    // --------------------------------------------------
    let partial_struct_def = partial_gen::gen_partial_struct(
        name,
        &partial_name,
        &input._original.vis,
        input.generics,
        &input.data,
    );
    let partial_struct_impl =
        partial_gen::gen_partial_impl(name, &partial_name, input.generics, &input.data);
    let try_from_partial_struct_impl =
        partial_gen::gen_try_from_partial_impl(name, &partial_name, input.generics);

    // --------------------------------------------------
    // partial implementations
    // --------------------------------------------------
    // this includes the MAIN LOGIC inside of resume partial
    // --------------------------------------------------
    let decode_partial_impl =
        decode_partial_gen::gen_decode_partial_impl(name, &partial_name, &stream, input.generics);
    let resume_partial_impl = decode_partial_gen::gen_resume_partial_impl(
        input,
        name,
        &partial_name,
        &stream,
        key_decoder,
        len_decoder,
    );

    // --------------------------------------------------
    // main decoder functions
    // --------------------------------------------------
    let decoder_fn_impl = gen_decoder_fn(
        name,
        &partial_name,
        &stream_lifetimed,
        lifetime,
        input.generics,
    );
    let decode_value_impl = gen_decode_value_impl(name, &partial_name, &stream, input.generics);

    // --------------------------------------------------
    // return em all
    // --------------------------------------------------
    quote! {
        #seek_if_sentinel
        #partial_struct_def
        #partial_struct_impl
        #try_from_partial_struct_impl
        #decoder_fn_impl
        #resume_partial_impl
        #decode_partial_impl
        #decode_value_impl
    }
}

/// Generates a trait-less implementation for idiomatic getting
/// of a partial decoder
///
/// This is used in other impls rather than having a complicated
/// constructor
fn gen_decoder_fn(
    name: &syn::Ident,
    partial_name: &syn::Ident,
    stream_lifetimed: &syn::Type,
    lifetime: proc_macro2::TokenStream,
    generics: &syn::Generics,
) -> proc_macro2::TokenStream {
    // --------------------------------------------------
    // generics
    // --------------------------------------------------
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // --------------------------------------------------
    // return
    // --------------------------------------------------
    quote! {
        #[automatically_derived]
        impl #impl_generics #name #ty_generics #where_clause {
            #[inline(always)]
            #[doc = concat!(" Construct a streaming [`tinyklv::Decoder`] for [`", stringify!(#name), "`].")]
            pub fn decoder<#lifetime>() -> ::tinyklv::Decoder<#partial_name #ty_generics , #stream_lifetimed> {
                ::tinyklv::Decoder::new()
            }
        }
    }
}

/// Generates the [`tinyklv::traits::DecodeValue`] implementation for a given type
///
/// This simply calls `decode_partial` and performs the following match / mapping:
///
/// * Ok and ready -> return decoded
/// * Ok but needs more -> attempt to coerce from partial to final, return Result
/// * Err -> return Err
fn gen_decode_value_impl(
    name: &syn::Ident,
    partial_name: &syn::Ident,
    stream: &syn::Type,
    generics: &syn::Generics,
) -> proc_macro2::TokenStream {
    // --------------------------------------------------
    // generics
    // --------------------------------------------------
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // --------------------------------------------------
    // return
    // --------------------------------------------------
    quote! {
        #[doc(hidden)]
        #[automatically_derived]
        #[doc = concat!(" [`", stringify!(#name), "`] implementation of [`tinyklv::prelude::DecodeValue`] for [`", stringify!(#stream), "`].")]
        impl #impl_generics ::tinyklv::traits::DecodeValue<#stream> for #name #ty_generics #where_clause {
            fn decode_value(input: &mut #stream) -> ::tinyklv::__export::winnow::Result<Self> {
                let checkpoint = input.checkpoint();
                match <Self as ::tinyklv::traits::DecodePartial<#stream>>::decode_partial(input) {
                    Ok(::tinyklv::decoder::Packet::Ready(v)) => Ok(v),
                    Ok(::tinyklv::decoder::Packet::NeedMore(p)) => match <#partial_name #ty_generics as ::tinyklv::traits::Partial>::finalize(p) {
                        Ok(v) => Ok(v),
                        Err(label) => Err(
                            ::tinyklv::__export::winnow::error::ContextError::new()
                                .add_context(
                                    input,
                                    &checkpoint,
                                    ::tinyklv::__export::winnow::error::StrContext::Label(label),
                                )
                        ),
                    },
                    Err(label) => Err(
                        ::tinyklv::__export::winnow::error::ContextError::new()
                            .add_context(
                                input,
                                &checkpoint,
                                ::tinyklv::__export::winnow::error::StrContext::Label(label),
                            )
                    ),
                }
            }
        }
    }
}
