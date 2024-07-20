// --------------------------------------------------
// external
// --------------------------------------------------
use quote::{
    quote,
    ToTokens,
};
use syn::{
    Meta,
    Data,
    Attribute,
    DataStruct,
    DeriveInput,
    parse_macro_input,
};
use thiserror::Error;
use proc_macro::TokenStream;

// --------------------------------------------------
// local
// --------------------------------------------------
mod klv;
use klv::*;
mod primitives;
mod nonlit2lit;
use primitives::Push;

#[derive(Error, Debug)]
enum Error {
    #[error("`{0}` can only be derived for structs.")]
    DeriveForNonStruct(String),
    #[error("Missing required attribute: function in `#[{0}(func = ?)]`.")]
    MissingFunc(String),
    #[error("Missing required attribute: type in `#[{0}(typ = ?)]`.")]
    MissingType(String),
    #[error("Attemping to parse non-integer value for `len`: {0}")]
    NonIntLength(String),
    #[error("Attemping to parse non-byte string for `key`: {0}")]
    NonByteStrKey(String),
    #[error("Encoder type mismatch: `#[encoder(typ = {0})]`, but expected {1} from variant `{2}`.")]
    EncoderTypeMismatch(String, String, String),
    #[error("Decoder type mismatch: `#[decoder(typ = {0})]`, but expected {1} from variant `{2}`.")]
    DecoderTypeMismatch(String, String, String),
}

const NAME: &str = "Klv";
#[proc_macro_derive(Klv, attributes(
    key_encoder,
    key_decoder,
    len_encoder,
    len_decoder,
    default_encoder,
    default_decoder,
    key,
    len,
    encoder,
    decoder
))]
pub fn klv(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    // --------------------------------------------------
    // extract the name, variants, and values
    // --------------------------------------------------
    let struct_name = &input.ident;
    let fields = match input.data {
        Data::Struct(DataStruct { fields, .. }) => fields,
        _ => panic!("{}", Error::DeriveForNonStruct(NAME.into())),
    };
    // --------------------------------------------------
    // parse struct-level attributes
    // --------------------------------------------------
    let mut struct_attrs = KlvStructAttr::default();
    input
        .attrs
        .iter()
        .filter(|attr|
            match attr.path.get_ident() {
                Some(ident) => KlvStructAttrValue::try_from(ident.to_string().as_str()).is_ok(),
                None => false,
            }
        )
        .for_each(|attr| parse_struct_attr(attr, &mut struct_attrs));
    // --------------------------------------------------
    // parse field-level attributes
    // --------------------------------------------------
    let mut field_attrs: Vec<_> = fields
        .iter()
        .filter_map(|field| {
            let mut attrs = KlvFieldAttr::default();
            field
                .attrs
                .iter()
                .for_each(|attr| parse_field_attr(attr, &mut attrs));
            attrs.name = field.ident.clone();
            attrs.typ = Some(field.ty.clone());
            Some(attrs)
        })
        .collect();
    // --------------------------------------------------
    // loop through fields, update encoder / decoder
    // using default types and struct attributes for
    // default encoder / decoder, if needed
    // --------------------------------------------------
    field_attrs
        .iter_mut()
        .for_each(|field_attr| {
            // --------------------------------------------------
            // if field has no encoder, AND the
            // default encoder exists for that type
            // within struct_attrs, use that
            // --------------------------------------------------
            // if field has no type, use the type
            // from the variant definition. otherwise,
            // if there is a mismatch, raise an error
            // --------------------------------------------------
            if field_attr.enc.is_none() {
                match field_attr.typ.as_ref() {
                    Some(typ) => match struct_attrs.default_enc.get(&typ.to_token_stream().to_string()) {
                        Some(func) => field_attr.enc = Some(func.clone()),
                        None => (),
                    },
                    None => (),
                }
            } else if let Some(enc_typ) = &field_attr.enc.as_ref().unwrap().typ {
                let enc_typ_str = enc_typ.to_token_stream().to_string();
                let variant_typ_str = field_attr.typ.as_ref().unwrap().to_token_stream().to_string();
                let variant_name_str = field_attr.name.as_ref().unwrap().to_token_stream().to_string();
                if enc_typ_str != variant_typ_str { panic!("{}", Error::EncoderTypeMismatch(enc_typ_str, variant_typ_str, variant_name_str)); }
            } else {
                field_attr.enc.as_mut().unwrap().typ = Some(field_attr.typ.as_ref().unwrap().clone());
            }
            // --------------------------------------------------
            // if field has no decoder, AND the
            // default decoder exists for that type
            // within struct_attrs, use that
            // --------------------------------------------------
            // if field has no type, use the type
            // from the variant definition. otherwise,
            // if there is a mismatch, raise an error
            // --------------------------------------------------
            if field_attr.dec.is_none() {
                match field_attr.typ.as_ref() {
                    Some(typ) => match struct_attrs.default_dec.get(&typ.to_token_stream().to_string()) {
                        Some(func) => field_attr.dec = Some(func.clone()),
                        None => (),
                    },
                    None => (),
                }
            } else if let Some(dec_typ) = &field_attr.dec.as_ref().unwrap().typ {
                let dec_typ_str = dec_typ.to_token_stream().to_string();
                let variant_typ_str = field_attr.typ.as_ref().unwrap().to_token_stream().to_string();
                let variant_name_str = field_attr.name.as_ref().unwrap().to_token_stream().to_string();
                if dec_typ_str != variant_typ_str { panic!("{}", Error::DecoderTypeMismatch(dec_typ_str, variant_typ_str, variant_name_str)); }
            } else {
                field_attr.dec.as_mut().unwrap().typ = Some(field_attr.typ.as_ref().unwrap().clone());
            }
        });
    // --------------------------------------------------
    // debug
    // --------------------------------------------------
    println!("{:#?}", struct_attrs);
    println!("{:#?}", field_attrs);
    // --------------------------------------------------
    // generate code
    // --------------------------------------------------
    // let expanded = quote! {
    //     // #[derive(Clone, Copy)]
    //     // #input
    //     // #field_attrs
    //     // #struct_attrs
    // };
    // TokenStream::from(expanded)
    unimplemented!()
}

/// Parses a struct-level attribute and pushes it to the
/// [`KlvStructAttr`] struct
fn parse_struct_attr(attr: &Attribute, struct_attrs: &mut KlvStructAttr) {
    let sattr = match nonlit2lit::ListedAttr::new(attr.to_token_stream().to_string()) {
        Ok(sattr) => sattr,
        _ => return,
    };
    struct_attrs.push(sattr);
}

/// Parses a field-level attribute and pushes it to the
/// [`KlvFieldAttr`] struct
fn parse_field_attr(attr: &Attribute, field_attrs: &mut KlvFieldAttr) {
    match attr.parse_meta() {
        Ok(Meta::NameValue(mnv)) => {
            let kvp = match nonlit2lit::KeyValPair::try_from(mnv) {
                Ok(kvp) => kvp,
                Err(e) => panic!("{}", e),
            };
            field_attrs.push(kvp);
        },
        Err(_) => match nonlit2lit::ListedAttr::new(attr.to_token_stream().to_string()) {
            Ok(sattr) => field_attrs.push(sattr),
            _ => return,
        }
        _ => return,
    }
}