//! Parsing utilities for proc-macro use in the [`tinyklv_impl`](crate) crate

/// Returns the inner type of an [`Option`], if it exists
pub(crate) fn unwrap_option_type(ty: &syn::Type) -> Option<&syn::Type> {
    is_option_helper(ty).1
}

/// Returns [`bool`] if [`syn::Type`] is an [`Option`]
pub(crate) fn is_option(ty: &syn::Type) -> bool {
    is_option_helper(ty).0
}

/// Helps determine if a [`syn::Type`] is an [`Option`] or not, with some
/// ancillary information. Used in [`crate::expand`]
fn is_option_helper(ty: &syn::Type) -> (bool, Option<&syn::Type>) {
    if let syn::Type::Path(syn::TypePath {
        path,
        ..
    }) = ty {
        if let Some(syn::PathSegment {
            ident: ref id,
            arguments: syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                args,
                ..
            })
        }) = path.segments.first() {
            if id == "Option" {
                return (true, args.first().and_then(|arg| match arg {
                    syn::GenericArgument::Type(inner_ty) => Some(inner_ty),
                    _ => None,
                }))
            }
        }
    }
    (false, None)
}

/// Inserts a lifetime into a type
pub(crate) fn insert_lifetime(ty: &syn::Type, lifetime_char: char) -> syn::Type {
    let lifetime = syn::Lifetime::new(&format!("'{lifetime_char}"), proc_macro2::Span::call_site());
    match ty {
        syn::Type::Reference(ty_ref) => syn::Type::Reference(syn::TypeReference {
            and_token: Default::default(),
            lifetime: Some(lifetime),
            mutability: ty_ref.mutability,
            elem: ty_ref.elem.clone(),
        }),
        _ => syn::Type::Reference(syn::TypeReference {
            and_token: Default::default(),
            lifetime: Some(lifetime),
            mutability: None,
            elem: Box::new(ty.clone()),
        })
    }
}

/// Default stream type, if not specified, for [`tinyklv`](crate) is `&[u8]`
pub(crate) fn u8_slice() -> syn::Type {
    syn::Type::Reference(syn::TypeReference {
        and_token: Default::default(),
        lifetime: None,
        mutability: None,
        elem: Box::new(syn::Type::Slice(syn::TypeSlice {
            bracket_token: Default::default(),
            elem: Box::new(syn::Type::Path(syn::TypePath {
                qself: None,
                path: syn::parse_quote! { u8 },
            })),
        })),
    })
}

/// Returns the element names + types of a struct without the `#[klv(..)]` attribute
pub(crate) fn elems_without_klv_attr(input: &syn::DeriveInput) -> Vec<(syn::Ident, syn::Type)> {
    // --------------------------------------------------
    // extract all field names
    // --------------------------------------------------
    let all_fields: Vec<(syn::Ident, syn::Type)> = match &input.data {
        syn::Data::Struct(data_struct) => {
            match data_struct.fields {
                syn::Fields::Named(ref fields) => fields.named.iter()
                    .map(|f| (
                        f.ident.as_ref().unwrap().clone(),
                        f.ty.clone(),
                    ))
                    .collect(),
                syn::Fields::Unnamed(_) => Vec::new(),
                syn::Fields::Unit => Vec::new(),
            }
        }
        _ => Vec::new(),
    };
    // --------------------------------------------------
    // extract all field names with klv attr
    // --------------------------------------------------
    let attr_fields: Vec<syn::Ident> = match &input.data {
        syn::Data::Struct(data_struct) => {
            match data_struct.fields {
                syn::Fields::Named(ref fields) => fields.named.iter()
                    .filter(|f| f.attrs.iter().any(|attr| attr.path.is_ident(crate::ATTR)))
                    .map(|f| f.ident.as_ref().unwrap().clone())
                    .collect(),
                syn::Fields::Unnamed(_) => Vec::new(),
                syn::Fields::Unit => Vec::new(),
            }
        }
        _ => Vec::new(),
    };
    // --------------------------------------------------
    // return non-attr fields by finding the difference
    // --------------------------------------------------
    all_fields
        .iter()
        .filter(|(name, _)| !attr_fields.contains(name))
        .cloned()
        .collect()
}

/// Converts a [`syn::Type`] to a [`proc_macro2::TokenStream`], using the
/// turbofish notation
/// 
/// For example:
/// 
/// * `Option<String>` -> `Option::<String>`
/// * `Option<Vec<u8>>` -> `Option::<Vec::<u8>>`
/// 
/// This is used to fill defaults when no `#[klv(..)]`
/// attribute is provided
/// 
/// For example:
/// 
/// ```rust no_run ignore
/// use tinyklv::Klv;
/// use tinyklv::prelude::*;
/// 
/// #[derive(Klv)]
/// #[klv(..)]
/// struct MyStruct {
///     // no #[klv(..)] attribute. defaults are used
///     field: Option<String>,
///     #[klv(key = 0x01)]
///     pub klv_field: Option<String>,
/// }
/// ```
/// 
/// When creating the `MyStruct` by decoding from a stream, it returns
/// a [`winnow::PResult<MyStruct>`]
/// 
/// During the decoding process, this is returned (see: [`crate::expand::gen_item_set`]):
/// 
/// ```rust no_run ignore
/// // parses from byte stream...
/// let klv_field_decoded = ...;
/// // return once parsed
/// return Ok(MyStruct {
///     field: Option::<String>::default(),
///     klv_field: klv_field_decoded,
/// });
/// ```
pub(crate) fn type2fish(ty: &syn::Type) -> proc_macro2::TokenStream {
    match ty {
        syn::Type::Path(type_path) => {
            let mut tokens = proc_macro2::TokenStream::new();
            for (i, segment) in type_path.path.segments.iter().enumerate() {
                if i > 0 { tokens.extend(quote::quote!(::)); }
                let ident = &segment.ident;
                tokens.extend(quote::quote!(#ident));
                match &segment.arguments {
                    syn::PathArguments::AngleBracketed(args) => {
                        let args_tokens: Vec<proc_macro2::TokenStream> = args.args.iter().map(|arg| {
                            match arg {
                                syn::GenericArgument::Type(ty) => type2fish(ty),
                                // extend this match to handle other [`syn::GenericArgument`] variants as needed
                                _ => quote::quote!(#arg),
                            }
                        }).collect();
                        if !args_tokens.is_empty() {
                            tokens.extend(quote::quote!(::<#(#args_tokens),*>));
                        }
                    },
                    // handle other [`syn::PathArguments`] variants if necessary
                    _ => {}
                }
            }
            tokens
        },
        // extend this match to handle other [`syn::Type`] variants as needed
        _ => quote::quote!(#ty),
    }
}