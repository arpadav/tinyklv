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
use crate::Error;
use crate::archive_ast::{
    self,
    // Input,
    KlvFieldAttr,
    KlvStructAttr,
};
use crate::kst;

/// Derive `Klv`
pub fn derive(input: &syn::DeriveInput) -> proc_macro::TokenStream {
    match kst::Input::from_syn(input) {
        Ok(parsed) => parsed.into(),
        Err(err) => panic!("{}", err),
    }
}

/// [From] implementation of [`proc_macro::TokenStream`] for [`kst::Input`]
impl From<kst::Input> for proc_macro::TokenStream {
    fn from(input: kst::Input) -> Self {
        unimplemented!();
    }
}

/// Derive `Klv` from [`Input`]
pub fn from(mut input: crate::archive_ast::Input) -> TokenStream {
    // let Input { name, sattr, fattrs } = input;
    // // --------------------------------------------------
    // // debug
    // // --------------------------------------------------
    // println!("{:#?}", sattr);
    // println!("{:#?}", fattrs);
    let impls = gen_xcoder_impls(&mut input);
    // --------------------------------------------------
    // generate code
    // --------------------------------------------------
    let expanded = quote! {
        #impls
        // #[derive(Clone, Copy)]
        // #input
        // #field_attrs
        // #struct_attrs
    };
    TokenStream::from(expanded)
}

/// Implementation of [`Functionify`] for all types that implement [`ToTokens`].
/// 
/// To be used to make trait names and function names more readable.
trait Functionify {
    fn functionify(&self) -> String;
}
impl<T: ToTokens> Functionify for T {
    fn functionify(&self) -> String {
        self
        .to_token_stream()
        .to_string()
        .chars()
        .map(|c| match c.is_alphanumeric() {
            true => c,
            false => '_',
        })
        .collect::<String>()
        .to_case(Case::Snake)
    }
}

enum Xcoder<T: ToString> {
    Encoder(T),
    Decoder(T),
    FixedDecoder(T, usize),
}
impl<T> Xcoder<T>
where
    T: ToString
{
    fn cnst(&self) -> String {
        match self {
            Xcoder::Encoder(_) => "Encoder".into(),
            Xcoder::Decoder(_) => "Decoder".into(),
            Xcoder::FixedDecoder(_, len) => format!("FixedDecoder{}", len),
        }
    }
    
    fn uid(&self) -> String {
        match self {
            Xcoder::Encoder(uid) => uid,
            Xcoder::Decoder(uid) => uid,
            Xcoder::FixedDecoder(uid, _) => uid,
        }.to_string()
    }

    fn get_trait_name(&self) -> syn::Ident {
        format_ident!("{}{}",
            self.uid().to_case(Case::Pascal),
            self.cnst().to_case(Case::Pascal),
        )
    }

    fn get_fn_name(&self) -> syn::Ident {
        format_ident!("{}_{}",
            self.uid().to_case(Case::Snake),
            self.cnst().to_case(Case::Snake),
        )
    }

    fn trait_definition(&self) -> TokenStream {
        let trait_name = self.get_trait_name();
        let fn_name = self.get_fn_name();
        match self {
            Xcoder::Encoder(_) => quote! {
                pub trait #trait_name<T> {
                    fn #fn_name(&self, input: T) -> Vec<u8>;
                }
            },
            Xcoder::Decoder(_) => quote! {
                pub trait #trait_name<T> {
                    fn #fn_name<'a>(&self, input: &'a [u8]) -> nom::IResult<&'a [u8], T>;
                }
            },
            Xcoder::FixedDecoder(_, _) => quote! {
                pub trait #trait_name<T> {
                    const LEN: usize;
                    fn #fn_name(&self, input: &[u8; Self::LEN]) -> T;
                }
            },
        }
    }

    fn implementation(&self, name: impl ToTokens, func: impl ToTokens, typ: impl ToTokens, include_self: bool) -> TokenStream {
        let name = name.into_token_stream();
        let func = func.into_token_stream();
        let typ = typ.into_token_stream();
        let trait_name = self.get_trait_name();
        let fn_name = self.get_fn_name();
        let is = is_ts(include_self);
        match self {
            Xcoder::Encoder(_) => quote! {
                impl #trait_name<#typ> for #name {
                    fn #fn_name(&self, input: #typ) -> Vec<u8> {
                        #func(#is input)
                    }
                }
            },
            Xcoder::Decoder(_) => quote! {
                impl #trait_name<#typ> for #name {
                    fn #fn_name<'a>(&self, input: &'a [u8]) -> nom::IResult<&'a [u8], #typ> {
                        #func(#is input)
                    }
                }
            },
            Xcoder::FixedDecoder(_, len) => quote! {
                impl #trait_name<#typ> for #name {
                    const LEN: usize = #len;
                    fn #fn_name(&self, input: &[u8; Self::LEN]) -> #typ {
                        #func(#is input)
                    }
                }
            },
        }
    }
}

fn gen_xcoder_impls(input: &mut crate::archive_ast::Input) -> proc_macro2::TokenStream {
    let archive_ast::Input { name, sattr, fattrs } = input;
    // --------------------------------------------------
    // key decoder
    // --------------------------------------------------
    let key_dec = sattr.key_dec.as_ref().unwrap();
    let (key_dec_ty, key_dec_fn, key_dec_is) = ty_fn_is(key_dec);
    let key_dec_ts = quote! {
        impl tinyklv::KeyDecoder<#key_dec_ty> for #name {
            fn key_decode<'a>(&self, input: &'a[u8]) -> nom::IResult<&'a[u8], #key_dec_ty> {
                match #key_dec_fn(#key_dec_is input) {
                    Ok((i, o)) => Ok((i, o.into())),
                    Err(e) => Err(e),
                }
            }
        }
    };
    // --------------------------------------------------
    // key encoder
    // --------------------------------------------------
    let key_enc = sattr.key_enc.as_ref().unwrap();
    let (key_enc_ty, key_enc_fn, key_enc_is) = ty_fn_is(key_enc);
    let key_enc_ts = quote! {
        impl tinyklv::KeyEncoder<#key_enc_ty> for #name {
            fn key_encode(&self, input: #key_enc_ty) -> Vec<u8> {
                #key_enc_fn(#key_enc_is input)
            }
        }
    };
    // --------------------------------------------------
    // len decoder
    // --------------------------------------------------
    let len_dec = sattr.len_dec.as_ref().unwrap();
    let (len_dec_ty, len_dec_fn, len_dec_is) = ty_fn_is(len_dec);
    let len_dec_ts = quote! {
        impl tinyklv::LenDecoder<#len_dec_ty> for #name {
            fn len_decode<'a>(&self, input: &'a[u8]) -> nom::IResult<&'a[u8], #len_dec_ty> {
                match #len_dec_fn(#len_dec_is input) {
                    Ok((i, o)) => Ok((i, o.into())),
                    Err(e) => Err(e),
                }
            }
        }
    };
    // --------------------------------------------------
    // len encoder
    // --------------------------------------------------
    let len_enc = sattr.len_enc.as_ref().unwrap();
    let (len_enc_ty, len_enc_fn, len_enc_is) = ty_fn_is(len_enc);
    let len_enc_ts = quote! {
        impl tinyklv::LenEncoder<#len_enc_ty> for #name {
            fn len_encode(&self, input: #len_enc_ty) -> Vec<u8> {
                #len_enc_fn(#len_enc_is input)
            }
        }
    };
    // --------------------------------------------------
    // get all unique required encoder and decoder
    // implementations
    // --------------------------------------------------
    let mut encoder_arg_set = HashSet::new();
    let mut decoder_arg_set = HashSet::new();
    fattrs
        .iter()
        .for_each(|field_attr| {
            encoder_arg_set.insert(field_attr.enc.as_ref().unwrap());
            decoder_arg_set.insert(field_attr.dec.as_ref().unwrap());
        });
    // --------------------------------------------------
    // loop through the field attributes
    // --------------------------------------------------
    let mut enc_pairs = Vec::new();
    let enc_imp = encoder_arg_set
        .iter()
        .map(|enc| {
            let (typ, func, include_self) = (
                enc.typ.as_ref().unwrap(),
                enc.func.as_ref().unwrap(),
                enc.include_self
            );
            let typ_string = typ.functionify();
            let func_string = func.functionify();
            let uid = format!("{}_{}", func_string, typ_string).to_case(Case::Snake);
            let enc_impl = Xcoder::Encoder(&uid);
            let efn = enc_impl.get_fn_name();
            enc_pairs.push((enc.clone(), efn));
            let eq = enc_impl.trait_definition();
            let ei = enc_impl.implementation(name.clone(), func, typ, include_self);
            quote! {
                #eq
                #ei
            }
        })
        .collect::<proc_macro2::TokenStream>();
    let mut dec_pairs = Vec::new();
    let dec_imp = decoder_arg_set
        .iter()
        .map(|dec| {
            let (typ, func, include_self) = (
                dec.typ.as_ref().unwrap(),
                dec.func.as_ref().unwrap(),
                dec.include_self
            );
            let typ_string = typ.functionify();
            let func_string = func.functionify();
            let uid = format!("{}_{}", func_string, typ_string).to_case(Case::Snake);
            let dec_impl = Xcoder::Decoder(&uid);
            let dfn = dec_impl.get_fn_name();
            dec_pairs.push((dec.clone(), dfn));
            let dq = dec_impl.trait_definition();
            let di = dec_impl.implementation(name.clone(), func, typ, include_self);
            quote! {
                #dq
                #di
            }
        })
        .collect::<proc_macro2::TokenStream>();
    return quote! {
        #key_dec_ts
        #key_enc_ts
        #len_dec_ts
        #len_enc_ts
        #enc_imp
        #dec_imp
    }
}

/// Type, Function, Include-Self -> TokenStream
fn ty_fn_is(item: &crate::archive_ast::KlvXcoderArg) -> (TokenStream, TokenStream, TokenStream) {
    let _ty = item.typ.to_token_stream();
    let _fn = item.func.to_token_stream();
    let _is = is_ts(item.include_self);
    (_ty, _fn, _is)
}

/// Include-Self -> TokenStream
fn is_ts(include_self: bool) -> proc_macro2::TokenStream {
    match include_self {
        true => quote! { &self, },
        false => quote! {},
    }
}