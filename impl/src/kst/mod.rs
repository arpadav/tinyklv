// --------------------------------------------------
// local
// --------------------------------------------------
pub(crate) use crate::kst::{
    field::FieldAttrSchema,
    strct::StructAttrSchema,
};
pub(crate) mod field;
pub(crate) mod strct;
pub(crate) mod xcoder;

/// [`Input`] of the [`crate::Klv`] derive macro
pub(crate) struct Input {
    pub name: syn::Ident,
    pub sattr: StructAttrSchema,
    pub fattrs: Vec<FieldAttrSchema>,
}

/// [`Input`] implementation of [`From`] for [`syn::DeriveInput`]
impl Input {
    pub fn from_syn(input: &syn::DeriveInput) -> Result<Self, crate::Error> {
        // --------------------------------------------------
        // extract the name, variants, and values
        // --------------------------------------------------
        let name = input.ident.clone();
        // --------------------------------------------------
        // get the struct attributes
        // --------------------------------------------------
        let sattr = match StructAttrSchema::from_syn(&input) {
            Some(sattr) => sattr,
            None => return Err(crate::Error::UnableToParseStructAttributes(name.to_string())),
        };
        // --------------------------------------------------
        // get the fields and their attributes
        // --------------------------------------------------
        let fields = match &input.data {
            syn::Data::Struct(syn::DataStruct { fields, .. }) => fields,
            _ => return Err(crate::Error::DeriveForNonStruct(crate::NAME.into(), name.to_string())),
        };
        let fattrs = fields
            .iter()
            .filter_map(|field| FieldAttrSchema::from_field(field))
            .collect::<Vec<_>>();
        Ok(Self { name, sattr, fattrs })
    }
}