// --------------------------------------------------
// local
// --------------------------------------------------
use quote::{
    quote,
    ToTokens,
    format_ident,
};
use convert_case::{
    Case,
    Casing,
};
use hashbrown::HashSet;
use proc_macro2::TokenStream;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::kst;
use crate::Error;

/// Derive `Klv`
pub fn derive(input: &syn::DeriveInput) -> proc_macro::TokenStream {
    let res: proc_macro::TokenStream = match kst::Input::from_syn(input) {
        Ok(parsed) => parsed.into(),
        // {
        //     // panic!("before");
        //     let res = parsed.into();
        //     // panic!("after");
        //     panic!("{:?}", res);
        //     res
        // },
        Err(err) => panic!("NOO {}", err),
    };
    // panic!("noooo");
    res
}

/// [From] implementation of [`proc_macro::TokenStream`] for [`kst::Input`]
impl From<kst::Input> for proc_macro::TokenStream {
    fn from(mut input: kst::Input) -> Self {
        let mut all_encoders_exist = true;
        let mut all_decoders_exist = true;
        for f in input.fattrs.iter_mut() {
            f.contents.enc = match f.contents.enc {
                Some(_) => continue,
                None => Some(symple::NameValue::new(match match input
                    .sattr
                    .defaults
                    .clone()
                    .into_iter()
                    .filter(|x| x.value.is_some())
                    .map(|x| {
                        let xcoder = x.value.unwrap();
                        (xcoder.ty, xcoder.xcoder)
                    })
                    .filter(|x| x.0 == f.ty)
                    .next() {
                        Some(x) => {
                            println!("{}", x.1);
                            x.1
                        },
                        None => {
                            all_encoders_exist = false;
                            break;
                            // panic!("no encoding allowed! not everything has an encoder"),
                        },
                    }.enc {
                        Some(x) => x,
                        None => {
                            all_encoders_exist = false;
                            break;
                            // panic!("no encoding allowed! not everything has an encoder"),
                        },
                    }
                )),
            };
            f.contents.dec = match f.contents.dec {
                Some(_) => continue,
                None => Some(symple::NameValue::new(match match input
                    .sattr
                    .defaults
                    .clone()
                    .into_iter()
                    .filter(|x| x.value.is_some())
                    .map(|x| {
                        let xcoder = x.value.unwrap();
                        (xcoder.ty, xcoder.xcoder)
                    })
                    .filter(|x| x.0 == f.ty)
                    .next() {
                        Some(x) => x.1,
                        None => {
                            all_decoders_exist = false;
                            break;
                            // panic!("no decoding allowed! not everything has a decoder"),
                        }, 
                    }.dec {
                        Some(x) => x,
                        None => {
                            all_decoders_exist = false;
                            break;
                            // panic!("no decoding allowed! not everything has a decoder"),
                        }, 
                    }
                )),
            };
        };
        println!("{:?}", input.fattrs);
        // proc_macro2::TokenStream::from("omg!".to_token_stream()).into()
        proc_macro2::TokenStream::from(quote! {
            struct IHaveExpanded;
        }).into()
    }
}