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
use proc_macro2::TokenStream;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::Error;
use crate::ast::{
    Input,
    KlvFieldAttr,
    KlvStructAttr,
    KlvStructAttrValue,
};

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
        let is = include_self_tokenstream(include_self);
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

/// Derive `Klv`
pub fn derive(input: &syn::DeriveInput) -> proc_macro::TokenStream {
    match Input::from_syn(input) {
        Ok(parsed) => parsed.into(),
        Err(err) => panic!("{}", err),
    }
}

/// Derive `Klv` from [`Input`]
pub fn from(input: crate::ast::Input) -> TokenStream {
    let Input { name, sattr, fattrs } = input;
    // --------------------------------------------------
    // debug
    // --------------------------------------------------
    // println!("{:#?}", sattr);
    println!("{:#?}", sattr.key_dec);
    // println!("{:#?}", fattrs);
    let something = gen_xcoder_impls(&name.to_token_stream(), &sattr, &fattrs);
    // --------------------------------------------------
    // generate code
    // --------------------------------------------------
    let expanded = quote! {
        #something
        // #[derive(Clone, Copy)]
        // #input
        // #field_attrs
        // #struct_attrs
    };
    TokenStream::from(expanded)
}

fn gen_xcoder_impls(struct_name: &proc_macro2::TokenStream, struct_attrs: &KlvStructAttr, field_attrs: &Vec<KlvFieldAttr>) -> proc_macro2::TokenStream {
    // --------------------------------------------------
    // key decoder
    // --------------------------------------------------
    let key_dec = struct_attrs.key_dec.as_ref().unwrap();
    let key_dec_ty = key_dec.typ.to_token_stream();
    let key_dec_fn = key_dec.func.to_token_stream();
    let key_dec_is = include_self_tokenstream(key_dec.include_self);
    let key_dec_ts = quote! {
        impl tinyklv::KeyDecoder<#key_dec_ty> for #struct_name {
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
    let key_enc = struct_attrs.key_enc.as_ref().unwrap();
    let key_enc_ty = key_enc.typ.to_token_stream();
    let key_enc_fn = key_enc.func.to_token_stream();
    let key_enc_is = include_self_tokenstream(key_enc.include_self);
    let key_enc_ts = quote! {
        impl tinyklv::KeyEncoder<#key_enc_ty> for #struct_name {
            fn key_encode(&self, input: #key_enc_ty) -> Vec<u8> {
                #key_enc_fn(#key_enc_is input)
            }
        }
    };
    // --------------------------------------------------
    // len decoder
    // --------------------------------------------------
    let len_dec = struct_attrs.len_dec.as_ref().unwrap();
    let len_dec_ty = len_dec.typ.to_token_stream();
    let len_dec_fn = len_dec.func.to_token_stream();
    let len_dec_is = include_self_tokenstream(len_dec.include_self);
    let len_dec_ts = quote! {
        impl tinyklv::LenDecoder<#len_dec_ty> for #struct_name {
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
    let len_enc = struct_attrs.len_enc.as_ref().unwrap();
    let len_enc_ty = len_enc.typ.to_token_stream();
    let len_enc_fn = len_enc.func.to_token_stream();
    let len_enc_is = include_self_tokenstream(len_enc.include_self);
    let len_enc_ts = quote! {
        impl tinyklv::LenEncoder<#len_enc_ty> for #struct_name {
            fn len_encode(&self, input: #len_enc_ty) -> Vec<u8> {
                #len_enc_fn(#len_enc_is input)
            }
        }
    };
    // --------------------------------------------------
    // loop through the field attributes
    // --------------------------------------------------
    let imp = field_attrs
        .iter()
        .map(|field_attr| {
            let enc = field_attr.enc.as_ref().unwrap();
            let (typ, func, include_self) = (
                enc.typ.as_ref().unwrap(),
                enc.func.as_ref().unwrap(),
                enc.include_self
            );

            let typ_string = typ.functionify();
            let func_string = func.functionify();
            let uid = format!("{}_{}", func_string, typ_string).to_case(Case::Snake);
            let eq = Xcoder::Encoder(&uid).trait_definition();
            let ei = Xcoder::Encoder(&uid).implementation(struct_name, func, typ, include_self);
            let dq = Xcoder::Decoder(&uid).trait_definition();
            let di = Xcoder::Decoder(&uid).implementation(struct_name, func, typ, include_self);
            quote! {
                #eq
                #ei
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
        #imp
    };
}

fn include_self_tokenstream(include_self: bool) -> proc_macro2::TokenStream {
    match include_self {
        true => quote! { &self, },
        false => quote! {},
    }
}