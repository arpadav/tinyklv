pub(crate) mod field;
pub(crate) mod strct;
pub(crate) mod xcoder;

// --------------------------------------------------
// external
// --------------------------------------------------
use thiserror::Error;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::kst::{
    field::FieldAttrSchema,
    strct::StructAttrSchema,
};

#[derive(Error, Debug)]
pub(crate) enum Error {
    #[error("Missing attribute BRUH")]
    Missing
}

pub(crate) struct Input {
    pub name: syn::Ident,
    pub sattr: StructAttrSchema,
    pub fattrs: Vec<FieldAttrSchema>,
}

/// [`Input`] implementation
impl Input {
    pub fn from_syn(input: &syn::DeriveInput) -> Result<Self, Error> {
        // --------------------------------------------------
        // extract the name, variants, and values
        // --------------------------------------------------
        let name = input.ident.clone();
        // --------------------------------------------------
        // get the struct attributes
        // --------------------------------------------------
        let sattr = match StructAttrSchema::from_syn(&input) {
            Some(sattr) => sattr,
            None => return Err(Error::Missing),
        };
        // --------------------------------------------------
        // get the fields and their attributes
        // --------------------------------------------------
        let fields = match &input.data {
            syn::Data::Struct(syn::DataStruct { fields, .. }) => fields,
            _ => panic!("{}", crate::Error::DeriveForNonStruct(crate::NAME.into(), name.to_string())),
        };
        let fattrs = fields
            .iter()
            .filter_map(|field| FieldAttrSchema::from_field(field))
            .collect::<Vec<_>>();
        Ok(Self { name, sattr, fattrs })
    }
}