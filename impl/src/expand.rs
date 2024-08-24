// --------------------------------------------------
// local
// --------------------------------------------------
use quote::quote;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::kst;
// use crate::Error;

/// Derive `Klv`
pub fn derive(input: &syn::DeriveInput) -> proc_macro::TokenStream {
    match kst::Input::from_syn(input) {
        Ok(parsed) => parsed.into(),
        Err(err) => panic!("{}", err),
    }
}

/// [From] implementation of [`proc_macro::TokenStream`] for [`kst::Input`]
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
        // check to see if all encoders / decoders exist
        // --------------------------------------------------
        let mut all_encoders_exist = true;
        let mut all_decoders_exist = true;
        for f in input.fattrs.iter() {
            all_encoders_exist &= f.contents.enc.is_some();
            all_decoders_exist &= f.contents.dec.is_some();
        }
        let mut expanded = quote! {};
        if all_decoders_exist {
            let decode_impl = gen_decode_impl(&input);
            expanded = quote! {
                #expanded
                #decode_impl
            }
        }
        // if all_encoders_exist {
        //     let encode_impl = gen_encode_impl(&input);
        //     expanded = quote! {
        //         #expanded
        //         #encode_impl
        //     }
        // }
        expanded.into()
    }
}

/// Generates the tokens for the entire [crate::prelude::StreamDecode] implementation
fn gen_decode_impl(input: &kst::Input) -> proc_macro2::TokenStream {
    let name = &input.name;
    // --------------------------------------------------
    // default stream -> &[u8]
    // --------------------------------------------------
    let stream = input.sattr.stream.value.clone().unwrap_or(crate::parse::u8_slice());
    let sentinel = input.sattr.sentinel.value.clone().unwrap_or_else(|| panic!("Sentinel is required"));
    let key_decoder = input.sattr.key.value.clone().unwrap_or_else(|| panic!("Key decoder is required")).dec;
    let len_decoder = input.sattr.len.value.clone().unwrap_or_else(|| panic!("Length decoder is required")).dec;
    let items_init = gen_items_init(&input.fattrs);
    let items_match = gen_items_match(&input.fattrs);
    let items_set = gen_item_set(name, &input.fattrs);
    let result = quote! {
        #[automatically_derived]
        #[doc = concat!(" [", stringify!(#name), "] implementation of [tinyklv::prelude::StreamDecode] for [", stringify!(#stream), "]")]
        impl ::tinyklv::prelude::StreamDecode<#stream> for #name {
            fn decode(input: &mut #stream) -> ::winnow::PResult<Self> {
                let checkpoint = input.checkpoint();
                let packet_len = match seq!(_:
                    #sentinel,
                    #len_decoder,
                ).parse_next(input) {
                    Ok(x) => x.0 as usize,
                    Err(e) => return Err(e.backtrack().add_context(
                        input,
                        &checkpoint,
                        winnow::error::StrContext::Label(concat!("Unable to find recognition sentinal and packet length for initial parsing of `", stringify!(#name), "` packet"))
                    )),
                };
                let mut packet = match take(packet_len).parse_next(input) {
                    Ok(x) => x,
                    Err(e) => match e.is_incomplete() {
                        true => return Err(winnow::error::ErrMode::Incomplete(winnow::error::Needed::Unknown)),
                        false => return Err(e.backtrack()),
                    },
                };
                let packet: &mut #stream = &mut packet;
                let checkpoint = input.checkpoint();
                #items_init
                loop {
                    match (
                        #key_decoder,
                        #len_decoder,
                    ).parse_next(packet) {
                        Ok((key, len)) => match (key, len) {
                            #items_match
                            (_, len) => { let _ = take::<usize, #stream, winnow::error::ContextError>(len).parse_next(packet); },
                        },
                        Err(_) => break,
                    }
                }
                #items_set
            }
        }
    };
    println!("{}", result);
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
/// `(#key, #optional_len) => #name = #dec (packet #optional_len_arg).ok(),`
fn gen_items_match(fatts: &Vec<kst::FieldAttrSchema>) -> proc_macro2::TokenStream {
    let arms = fatts.iter().map(|field| {
        let key = &field.contents.key.value.clone().unwrap_or_else(|| panic!("Key is required"));
        let name = &field.name;
        let dec = field.contents.dec.clone().unwrap_or_else(|| panic!("Decoder is required")).value.unwrap_or_else(|| panic!("Decoder is required"));
        let dynlen = field.contents.dynlen;
        let optional_len = if let Some(true) = dynlen { quote! { len } } else { quote! { _ } };
        let optional_len_arg = if let Some(true) = dynlen { quote! { , len } } else { quote! {} };
        quote! {
            (#key, #optional_len) => #name = #dec (packet #optional_len_arg).ok(),
        }
    });
    quote! {
        #(#arms)*
    }
}

/// Generates the tokens for setting the field variables upon returning of the output struct
/// 
/// `Ok(#struct_name { #(#field_set_on_return)* })`
fn gen_item_set(struct_name: &syn::Ident, fatts: &Vec<kst::FieldAttrSchema>) -> proc_macro2::TokenStream {
    let field_set_on_return = fatts.iter().map(|field| {
        let kst::FieldAttrSchema { name, ty, .. } = field;
        match crate::parse::is_option(ty) {
            false => quote! {
                #name: #name.ok_or_else(|| {
                    winnow::error::ErrMode::Backtrack(winnow::error::ContextError::new().add_context(
                        input,
                        &checkpoint,
                        winnow::error::StrContext::Label(concat!("`", stringify!(#name), "` is a required value missing from the `", stringify!(#struct_name), "` packet. To prevent this, set this field as optional."))
                    ))
                })?,
            },
            true => quote! { #name, },
        }
    });
    quote! {
        Ok(#struct_name {
            #(#field_set_on_return)*
        })
    }
}
