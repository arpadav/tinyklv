//! Parsing utilities for proc-macro use in the [`tinyklv_impl`](crate) crate
//!
//! Provides small helper functions shared across the encode and decode
//! code-generation passes: `Option` detection and inner-type extraction,
//! lifetime insertion into reference types, the default `&[u8]` stream
//! type constructor, and turbofish-notation type serialization
//!
//! Author: aav

/// Returns the inner type `T` of an `Option<T>`, if the type is an `Option`
///
/// Delegates to [`is_option_helper`]. Returns `None` if `ty` is not an
/// `Option` or if the generic argument is not a plain type
///
/// # Arguments
///
/// * `ty` - The [`syn::Type`] to inspect
///
/// # Returns
///
/// `Some(&inner_ty)` when `ty` is `Option<inner_ty>`, or `None` otherwise
pub(crate) fn unwrap_option_type(ty: &syn::Type) -> Option<&syn::Type> {
    is_option_helper(ty).1
}

/// Returns `true` if a [`syn::Type`] is an `Option<T>`
///
/// Delegates to [`is_option_helper`], discarding the inner-type result
///
/// # Arguments
///
/// * `ty` - The [`syn::Type`] to inspect
///
/// # Returns
///
/// `true` when `ty` is `Option<..>`, `false` otherwise
pub(crate) fn is_option(ty: &syn::Type) -> bool {
    is_option_helper(ty).0
}

/// Determines whether a [`syn::Type`] is an `Option` and extracts its inner type
///
/// Checks whether `ty` is a single-segment path whose ident is `Option` and
/// whose first generic argument is a type. Returns both the boolean flag and
/// the optional inner type so callers can use either piece without a second pass
///
/// # Arguments
///
/// * `ty` - The [`syn::Type`] to inspect
///
/// # Returns
///
/// A tuple `(is_option, inner)` where `is_option` is `true` when `ty` is
/// `Option<..>`, and `inner` is `Some(&T)` for `Option<T>` or `None` otherwise
fn is_option_helper(ty: &syn::Type) -> (bool, Option<&syn::Type>) {
    if let syn::Type::Path(syn::TypePath { path, .. }) = ty
        && let Some(syn::PathSegment {
            ident: id,
            arguments:
                syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments { args, .. }),
        }) = path.segments.first()
        && id == "Option"
    {
        return (
            true,
            args.first().and_then(|arg| match arg {
                syn::GenericArgument::Type(inner_ty) => Some(inner_ty),
                _ => None,
            }),
        );
    }
    (false, None)
}

/// Inserts a named lifetime into a [`syn::Type`], producing a reference type
///
/// When `ty` is already a reference (`&T` or `&mut T`), the lifetime is placed
/// on the existing reference. When `ty` is any other form (e.g. `[u8]`, a path
/// type), it is wrapped in a new shared reference `&'lifetime ty`. This is used
/// to produce the stream-with-lifetime form required by `SeekSentinel` and
/// `decoder()` function signatures
///
/// # Arguments
///
/// * `ty` - The base type to attach the lifetime to
/// * `lifetime_char` - A token stream containing the lifetime token, e.g. `'z`
///
/// # Returns
///
/// A new [`syn::Type`] with the named lifetime attached
pub(crate) fn insert_lifetime(
    ty: &syn::Type,
    lifetime_char: proc_macro2::TokenStream,
) -> syn::Type {
    let lifetime = syn::Lifetime::new(
        lifetime_char.to_string().as_str(),
        proc_macro2::Span::call_site(),
    );
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
        }),
    }
}

/// Constructs the default stream type `&[u8]` as a [`syn::Type`]
///
/// Used throughout the decode codegen when the container's `stream = ..`
/// attribute is absent. Builds the type programmatically to avoid a
/// `syn::parse_str` call that would require an `unwrap`
///
/// # Returns
///
/// A [`syn::Type`] equivalent to `&[u8]`
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
/// a [`winnow::Result<MyStruct>`]
///
/// During the decoding process, this is returned (see: [`crate::expand::gen_item_set`]):
///
/// ```rust no_run ignore
/// // parses from byte stream..
/// let klv_field_decoded = ...;
/// // return once parsed
/// return Ok(MyStruct {
///     field: Option::<String>::default(),
///     klv_field: klv_field_decoded,
/// });
/// ```
pub(crate) fn type2fish(ty: &syn::Type) -> proc_macro2::TokenStream {
    // extend this to handle other [`syn::Type`] variants as needed
    let syn::Type::Path(type_path) = ty else {
        return quote::quote!(#ty);
    };
    let mut tokens = proc_macro2::TokenStream::new();
    for (i, segment) in type_path.path.segments.iter().enumerate() {
        if i > 0 {
            tokens.extend(quote::quote!(::));
        }
        let ident = &segment.ident;
        tokens.extend(quote::quote!(#ident));
        if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
            let args_tokens: Vec<proc_macro2::TokenStream> = args
                .args
                .iter()
                .map(|arg| {
                    if let syn::GenericArgument::Type(ty) = arg {
                        type2fish(ty)
                    } else {
                        // extend this to handle other [`syn::GenericArgument`] variants as needed
                        quote::quote!(#arg)
                    }
                })
                .collect();
            if !args_tokens.is_empty() {
                tokens.extend(quote::quote!(::<#(#args_tokens),*>));
            }
        }
    }
    tokens
}
