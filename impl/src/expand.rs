// --------------------------------------------------
// local
// --------------------------------------------------
use quote::quote;
use tinyklv_common::symple::prelude::*;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::kst;
use crate::kst::xcoder::PathLike;

// --------------------------------------------------
// constants
// --------------------------------------------------
const PACKET_LIFETIME_CHAR: char = 'z';

/// Derive [`crate::Klv`]
pub fn derive(input: &syn::DeriveInput) -> proc_macro::TokenStream {
    match kst::Input::from_syn(input) {
        Ok(parsed) => parsed.into(),
        Err(err) => panic!("{}", err),
    }
}

/// [`From`] implementation of [`proc_macro::TokenStream`] for [`kst::Input`]
impl From<kst::Input> for proc_macro::TokenStream {
    fn from(mut input: kst::Input) -> Self {
        // --------------------------------------------------
        // set all None enc/dec fields to provided defaults
        // --------------------------------------------------
        for f in input.fattrs.iter_mut() {
            input
                .sattr
                .defaults
                .clone()
                .into_iter()
                .filter(|x| x.value.is_some())
                .for_each(|x| f.contents.update(&f.ty, &x));
        }
        // --------------------------------------------------
        // set default stream to &[u8], if not set
        // --------------------------------------------------
        // if match input.sattr.stream {
        //     Some(_) => (),
        //     None => input.sattr.stream.v
        // }
        // --------------------------------------------------
        // check to see if all encoders / decoders exist
        // --------------------------------------------------
        let mut all_encoders_exist = true;
        let mut all_decoders_exist = true;
        for f in input.fattrs.iter() {
            all_encoders_exist &= f.contents.enc().is_some();
            all_decoders_exist &= f.contents.dec().is_some();
        }
        let mut expanded = quote! {};
        if all_decoders_exist {
            let decode_impl = gen_decode_impl(&input);
            expanded = quote! {
                #expanded
                #decode_impl
            }
        }
        if all_encoders_exist {
            let encode_impl = gen_encode_impl(&input);
            expanded = quote! {
                #expanded
                #encode_impl
            }
        }
        // println!("{}", input.sattr);
        // println!("{:?}", input.fattrs);
        println!("{}", expanded);
        expanded.into()
    }
}

/// Generates the tokens for the entire [`tinyklv::prelude::Encode`](https://docs.rs/tinyklv/latest/tinyklv/prelude/trait.Encode.html) implementation
fn gen_encode_impl(input: &kst::Input) -> proc_macro2::TokenStream {
    let name = &input.name;
    // --------------------------------------------------
    // default stream -> &[u8]
    // --------------------------------------------------
    let sentinel = input.sattr.sentinel.as_ref().map_or(None, |x| x.get().clone());
    let key_encoder = input
        .sattr.key.value.clone()
        .unwrap_or_else(|| panic!("{}", crate::Error::MissingFunc("struct".into(), "key".into(), "enc".into(), "encoder".into())))
        .xcoder.enc
        .unwrap_or_else(|| panic!("{}", crate::Error::MissingFunc("struct".into(), "key".into(), "enc".into(), "encoder".into())));
    let len_encoder = input
        .sattr.len.value.clone()
        .unwrap_or_else(|| panic!("{}", crate::Error::MissingFunc("struct".into(), "len".into(), "enc".into(), "encoder".into())))
        .xcoder.enc
        .unwrap_or_else(|| panic!("{}", crate::Error::MissingFunc("struct".into(), "len".into(), "enc".into(), "encoder".into())));

    let items_encoded = gen_items_encoded(&input, &key_encoder, &len_encoder);

    let encode_with_key_len = match sentinel {
        Some(sentinel) => quote! {
            #[automatically_derived]
            impl ::tinyklv::prelude::Encode<Vec<u8>> for #name {
                fn encode(&self) -> Vec<u8> {
                    self.encode_value().into_klv(
                        #key_encoder (#sentinel),
                        #len_encoder ,
                    )
                }
            }
        },
        None => quote! {}
    };

    quote! {
        #[automatically_derived]
        impl ::tinyklv::prelude::EncodeValue<Vec<u8>> for #name {
            fn encode_value(&self) -> Vec<u8> {
                let mut output = vec![];
                #items_encoded
                output
            }
        }
        #encode_with_key_len
    }
}

fn gen_items_encoded(input: &kst::Input, key_encoder: &PathLike, len_encoder: &PathLike) -> proc_macro2::TokenStream {
    let items_encoded = input.fattrs.iter().map(|field| {
        let name = &field.name;
        let value_encoder = field
            .contents.enc()
            .unwrap_or_else(|| panic!("{}", crate::Error::MissingFunc("struct".into(), "value".into(), "enc".into(), "encoder".into())));
        let key = field
            .contents.key
            .value.clone().unwrap_or_else(|| panic!("{}", crate::Error::MissingKey(name.to_string()))
        );
        quote! {
            output.extend(#value_encoder(&self.#name).into_klv(#key_encoder(#key), #len_encoder));
        }
    });
    quote! { #(#items_encoded)* }
}

/// Generates the tokens for the entire [`tinyklv::prelude::Decode`](https://docs.rs/tinyklv/latest/tinyklv/prelude/trait.Decode.html) implementation
fn gen_decode_impl(input: &kst::Input) -> proc_macro2::TokenStream {
    let name = &input.name;
    // --------------------------------------------------
    // default stream -> &[u8]
    // --------------------------------------------------
    let stream = input.sattr.stream.value.clone().unwrap_or(crate::parse::u8_slice());
    let stream_lifetimed = crate::parse::insert_lifetime(&stream, PACKET_LIFETIME_CHAR);
    let sentinel = input.sattr.sentinel.as_ref().map_or(None, |x| x.get().clone());
    let key_decoder = input
        .sattr.key.value.clone()
        .unwrap_or_else(|| panic!("{}", crate::Error::MissingFunc("struct".into(), "key".into(), "dec".into(), "decoder".into())))
        .xcoder.dec
        .unwrap_or_else(|| panic!("{}", crate::Error::MissingFunc("struct".into(), "key".into(), "dec".into(), "decoder".into())));
    let len_decoder = input
        .sattr.len.value.clone()
        .unwrap_or_else(|| panic!("{}", crate::Error::MissingFunc("struct".into(), "len".into(), "dec".into(), "decoder".into())))
        .xcoder.dec
        .unwrap_or_else(|| panic!("{}", crate::Error::MissingFunc("struct".into(), "len".into(), "dec".into(), "decoder".into())));
    let items_init = gen_items_init(&input.fattrs);
    let items_match = gen_items_match(&input.fattrs);
    let items_set = gen_item_set(name, &input.fattrs, crate::parse::elems_without_klv_attr(&input.input));
    let seek_if_sentinel = match sentinel {
        Some(sentinel) => quote! {
            #[automatically_derived]
            #[doc = concat!(" [`", stringify!(#name), "`] implementation of [`tinyklv::prelude::Seek`] for [`", stringify!(#stream), "`]")]
            impl ::tinyklv::prelude::Seek<#stream> for #name {
                // ---- vvv ---- remember this is PACKET_LIFETIME_CHAR
                fn seek<'z>(input: &mut #stream_lifetimed) -> ::tinyklv::reexport::winnow::PResult<#stream_lifetimed> {
                // ---- ^^^ ---- remember this is PACKET_LIFETIME_CHAR
                    let checkpoint = input.checkpoint();
                    let packet_len = match ::tinyklv::reexport::winnow::combinator::seq!(_:
                        // ::winnow::token::take_until(0.., #sentinel),
                        // ::winnow::token::take_till(0.., #sentinel),
                        #sentinel,
                        #len_decoder,
                    ).parse_next(input) {
                        Ok(x) => x.0 as usize,
                        Err(e) => return Err(e.backtrack().add_context(
                            input,
                            &checkpoint,
                            ::tinyklv::reexport::winnow::error::StrContext::Label(
                                concat!("Unable to find recognition sentinal and packet length for initial parsing of `", stringify!(#name), "` packet")
                            )
                        )),
                    };
                    ::tinyklv::reexport::winnow::token::take(packet_len).parse_next(input)
                }
            }
        },
        None => quote! {}
    };
    let result = quote! {
        #seek_if_sentinel
        #[automatically_derived]
        #[doc = concat!(" [`", stringify!(#name), "`] implementation of [`tinyklv::prelude::Decode`] for [`", stringify!(#stream), "`]")]
        impl ::tinyklv::prelude::Decode<#stream> for #name {
            fn decode(input: &mut #stream) -> ::tinyklv::reexport::winnow::PResult<Self> {
                let checkpoint = input.checkpoint();
                #items_init
                loop {
                    match (
                        #key_decoder,
                        #len_decoder,
                    ).parse_next(input) {
                        Ok((key, len)) => match (key, len) {
                            #items_match
                            (_, len) => { let _ = ::tinyklv::reexport::winnow::token::take::<usize, #stream, ::tinyklv::reexport::winnow::error::ContextError>(len).parse_next(input); },
                        },
                        Err(_) => break,
                    }
                }
                #items_set
            }
        }
    };
    // println!("{}", result);
    result
}

/// Generates the tokens for initializing the field variables as optional
/// 
/// `let mut #name: Option<#ty> = None;`
fn gen_items_init(fatts: &Vec<kst::FieldAttrSchema>) -> proc_macro2::TokenStream {
    let field_initializations = fatts.iter().map(|field| {
        let kst::FieldAttrSchema { name, ty, .. } = field;
        let ty = crate::parse::unwrap_option_type(ty).unwrap_or(ty);
        quote! { let mut #name: Option<#ty> = None; }
    });
    quote! { #(#field_initializations)* }
}

/// Generates the tokens for matching the key/len's with fields and parsers
/// 
/// `(#key, #optional_len) => #name = #dec (input #optional_len_arg).ok(),`
fn gen_items_match(fatts: &Vec<kst::FieldAttrSchema>) -> proc_macro2::TokenStream {
    let arms = fatts.iter().map(|field| {
        let name = &field.name;
        let key = &field.contents.key.value.clone().unwrap_or_else(||
            panic!("{}", crate::Error::MissingKey(name.to_string()))
        );
        let dec = field.contents.dec().clone().unwrap_or_else(||
            panic!("{}", crate::Error::MissingFunc(format!("field `{}`", name), "value".into(), "dec".into(), "decoder".into()))
        );
        let dynlen = field.contents.dynlen();
        let optional_len = if let Some(true) = dynlen { quote! { len } } else { quote! { _ } };
        let optional_len_arg = if let Some(true) = dynlen { quote! { (len) } } else { quote! {} };
        quote! {
            (#key, #optional_len) => #name = #dec #optional_len_arg (input).ok(),
        }
    });
    quote! {
        #(#arms)*
    }
}

/// Generates the tokens for setting the field variables upon returning of the output struct
/// 
/// `Ok(#struct_name { #(#field_set_on_return)* })`
fn gen_item_set(struct_name: &syn::Ident, fatts: &Vec<kst::FieldAttrSchema>, elem_name_type_without_klv: Vec<(syn::Ident, syn::Type)>) -> proc_macro2::TokenStream {
    let field_set_on_return = fatts.iter().map(|field| {
        let kst::FieldAttrSchema { name, ty, .. } = field;
        match crate::parse::is_option(ty) {
            false => quote! {
                #name: #name.ok_or_else(|| {
                    ::tinyklv::reexport::winnow::error::ErrMode::Backtrack(
                        ::tinyklv::reexport::winnow::error::ContextError::new().add_context(
                            input,
                            &checkpoint,
                            ::tinyklv::reexport::winnow::error::StrContext::Label(
                                concat!("`", stringify!(#name), "` is a required value missing from the `", stringify!(#struct_name), "` packet. To prevent this, set this field as optional.")
                            )
                        )
                    )
                })?,
            },
            true => quote! { #name, },
        }
    });
    // --------------------------------------------------
    // elements without the  `#[klv(..)]` attribute must
    // implement [`default::Default`]
    // --------------------------------------------------
    // if the default does not exist, then this will not compile
    // --------------------------------------------------
    match elem_name_type_without_klv.len() != 0 {
        false => quote! { Ok(#struct_name { #(#field_set_on_return)* }) },
        true => {
            let names: Vec<_> = elem_name_type_without_klv.iter().map(|(name, _)| name.clone()).collect();
            let types: Vec<_> = elem_name_type_without_klv.iter().map(|(_, ty)| crate::parse::type2fish(ty)).collect();
            let individual_defaults = quote! { #(#names: #types::default())*, };
            quote! {
                Ok(#struct_name {
                    #(#field_set_on_return)* 
                    #individual_defaults
                })
            }
        },
    }
}
