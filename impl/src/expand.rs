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
        println!("{:?}", input.sattr);
        println!("{:#?}", input.fattrs);
        let mut all_encoders_exist = true;
        let mut all_decoders_exist = true;
        for f in input.fattrs.iter_mut() {
            // --------------------------------------------------
            // fill default encoders
            // --------------------------------------------------
            if f.contents.enc.is_none() {
                f.contents.enc = (|| Some(symple::NameValue::new(
                    match match input
                        .sattr
                        .defaults
                        .clone()
                        .into_iter()
                        .filter(|x| x.value.is_some())
                        .map(|x| {
                            let xcoder = x.value.unwrap();
                            (xcoder.ty, xcoder.xcoder)
                        })
                        .filter(|x| x.0 == f.ty || match crate::parse::unwrap_option_type(&f.ty) {
                            Some(f) => &x.0 == f,
                            None => false,
                        })
                        .next() {
                            Some(x) => x.1,
                            None => {
                                all_encoders_exist = false;
                                return None;
                            }
                        }.enc {
                            Some(x) => x,
                            None => {
                                all_encoders_exist = false;
                                return None;
                            }
                        }
                    ))
                )();
            }
            // --------------------------------------------------
            // fill default decoders
            // --------------------------------------------------
            if f.contents.dec.is_none() {
                f.contents.dec = (|| Some(symple::NameValue::new(
                    match match input
                        .sattr
                        .defaults
                        .clone()
                        .into_iter()
                        .filter(|x| x.value.is_some())
                        .map(|x| {
                            let xcoder = x.value.unwrap();
                            (xcoder.ty, xcoder.xcoder)
                        })
                        .filter(|x| x.0 == f.ty || match crate::parse::unwrap_option_type(&f.ty) {
                            Some(f) => &x.0 == f,
                            None => false,
                        })
                        .next() {
                            Some(x) => x.1,
                            None => {
                                all_decoders_exist = false;
                                return None;
                            }
                        }.dec {
                            Some(x) => x,
                            None => {
                                all_decoders_exist = false;
                                return None;
                            }
                        }
                    ))
                )();
            }
        };
        println!("all_encoders_exist: {}", all_encoders_exist);
        println!("all_decoders_exist: {}", all_decoders_exist);
        println!("{:#?}", input.fattrs);
        // proc_macro2::TokenStream::from("omg!".to_token_stream()).into()
        proc_macro2::TokenStream::from(gen_decode_impl(&input)).into()
    }
}

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
    quote! {
        #[automatically_derived]
        #[doc = concat!("[", stringify!($name), "] implementation of [tinyklv::prelude::StreamDecode] for [", stringify!($stream), "]")]
        impl ::tinyklv::prelude::StreamDecode<#stream> for #name {
            fn decode(input: &mut #stream) -> ::winnow::PResult<Self> {
                let checkpoint = input.checkpoint();
                let packet_len = seq!(_:
                    #sentinel,
                    #len_decoder
                ).parse_next(input)?.0 as usize;
                let mut packet = take(packet_len).parse_next(input)?;
                let packet: &mut $stream = &mut packet;
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
            }
        }
    }
}

fn gen_items_init(fatts: &Vec<kst::FieldAttrSchema>) -> proc_macro2::TokenStream {
    let field_initializations = fatts.iter().map(|field| {
        let kst::FieldAttrSchema { name, ty, .. } = field;
        let ty = crate::parse::unwrap_option_type(ty).unwrap_or(ty);
        quote! { let mut #name: Option<#ty> = None; }
    });
    quote! { #(#field_initializations)* }
}

fn gen_items_match(fatts: &Vec<kst::FieldAttrSchema>) -> proc_macro2::TokenStream {
    let arms = fatts.iter().map(|field| {
        let key = &field.contents.key.value.clone().unwrap_or_else(|| panic!("Key is required"));
        let name = &field.name;
        let dec = field.contents.dec.clone().unwrap_or_else(|| panic!("Decoder is required")).value.unwrap_or_else(|| panic!("Decoder is required"));
        let dynlen = field.contents.dynlen;
        let optional_len = if dynlen { quote! { len } } else { quote! { _ } };
        let optional_len_arg = if dynlen { quote! { , len } } else { quote! {} };
        quote! {
            (#key, #optional_len) => #name = #dec(packet #optional_len_arg).ok(),
        }
    });
    quote! {
        #(#arms)*
    }
}