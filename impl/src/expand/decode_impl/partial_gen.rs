//! Code generation for the partial-packet struct and its trait implementations
//!
//! Provides three generators consumed by [`super::gen_decode_impl`]:
//!
//! * [`gen_partial_struct`] - emits the hidden `XxxPartialPacket` struct (a
//!   mirror of the user struct with every KLV field as `Option<T>`) and its
//!   manual [`Default`] impl seeded by any field-level `default` attributes
//! * [`gen_partial_impl`] - emits the [`tinyklv::traits::Partial`] impl,
//!   which validates required fields and constructs the final struct via
//!   `Partial::finalize`
//! * [`gen_try_from_partial_impl`] - emits a `TryFrom<XxxPartialPacket> for
//!   Xxx` convenience impl that routes through `Partial::finalize`
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::{
    ast::{attr::MainField, types},
    expand::helpers,
    symbol,
};

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::{ToTokens, quote};

/// Generates the partial-packet struct definition and its manual [`Default`] impl
///
/// The emitted struct mirrors the main struct with one `Option<T>` field per
/// KLV-annotated field; non-KLV fields are omitted from the partial and filled
/// via `<T>::default()` only at finalization time. Generic parameters are
/// forwarded as-is; a `PhantomData` field is added when the original struct
/// carries type or lifetime parameters so the compiler does not reject the
/// partial struct for unused generics
///
/// The `Default` impl seeds each field according to its `default` attribute:
///
/// * no `default` attribute  -> `Option::<T>::None`
/// * bare `default`          -> `Some(<T as ::core::default::Default>::default())`
/// * `default = <expr>`      -> `Some(#expr)`
///
/// A manual `Default` impl (rather than `#[derive(Default)]`) is required
/// because `default = <expr>` may be an arbitrary expression that does not
/// match `<T as Default>::default()`
///
/// # Arguments
///
/// * `name` - The main struct ident, used in the generated doc comment
/// * `partial_name` - The partial struct ident to emit (`XxxPartialPacket`)
/// * `vis` - The visibility of the original struct, applied to the partial as well
/// * `generics` - Generic parameters from the original struct definition
/// * `fatts` - All fields of the container; non-KLV fields are skipped
///
/// # Returns
///
/// A [`proc_macro2::TokenStream`] containing the partial struct definition and
/// its `Default` impl block
pub(super) fn gen_partial_struct(
    name: &syn::Ident,
    partial_name: &syn::Ident,
    vis: &syn::Visibility,
    generics: &syn::Generics,
    fatts: &[MainField],
) -> proc_macro2::TokenStream {
    // --------------------------------------------------
    // split generics
    // --------------------------------------------------
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    // --------------------------------------------------
    // Option<T> fields flattened: store Option<T>, not Option<Option<T>>
    // --------------------------------------------------
    let field_decls = fatts.iter().filter_map(|field| {
        field.attrs.as_ref()?;
        let MainField {
            name: field_name,
            ty,
            ..
        } = field;
        let ty = helpers::unwrap_option_type(ty).unwrap_or(ty);
        // --------------------------------------------------
        // `pub #field_name: Option<#ty>,` per field
        // --------------------------------------------------
        Some(quote! {
            pub #field_name: ::core::option::Option<#ty>,
        })
    });

    // --------------------------------------------------
    // user structs may carry type params that only appear in non-klv fields
    // therefore, must extract type, lifetime, and other params
    // to put them into a phantom data
    // --------------------------------------------------
    // this solves E0392 https://doc.rust-lang.org/error_codes/E0392.html
    // --------------------------------------------------
    let type_params: Vec<&syn::Ident> = generics
        .params
        .iter()
        .filter_map(|p| match p {
            syn::GenericParam::Type(tp) => Some(&tp.ident),
            _ => None,
        })
        .collect();
    let lifetime_params: Vec<&syn::Lifetime> = generics
        .params
        .iter()
        .filter_map(|p| match p {
            syn::GenericParam::Lifetime(lp) => Some(&lp.lifetime),
            _ => None,
        })
        .collect();
    // --------------------------------------------------
    // phantom field for struct def and its default initializer
    // --------------------------------------------------
    let (phantom_field, phantom_default) = if type_params.is_empty() && lifetime_params.is_empty() {
        (quote! {}, quote! {})
    } else {
        let lt_refs = lifetime_params.iter().map(|lt| quote! { & #lt () });
        (
            quote! {
                #[doc(hidden)]
                pub __tinyklv_phantom: ::core::marker::PhantomData<(
                    #(#lt_refs,)*
                    #(#type_params,)*
                )>,
            },
            quote! {
                #[doc(hidden)]
                __tinyklv_phantom: ::core::marker::PhantomData,
            },
        )
    };
    // --------------------------------------------------
    // get the default inits for the fields
    // --------------------------------------------------
    let default_inits = fatts.iter().filter_map(|field| {
        let attrs = field.attrs.as_ref()?;
        let MainField {
            name: field_name,
            ty,
            ..
        } = field;
        let default = attrs.default.clone();
        let ty = helpers::unwrap_option_type(ty).unwrap_or(ty);
        // --------------------------------------------------
        // the DefaultValue is indicated by tinyklv field attributes,
        // which are used here to help with init
        // --------------------------------------------------
        let init = match default {
            // --------------------------------------------------
            // #[klv(default = ..)]
            // --------------------------------------------------
            Some(types::DefaultValue::Expr(expr)) => quote! {
                ::core::option::Option::<#ty>::Some(#expr)
            },
            // --------------------------------------------------
            // #[klv(default)]
            // --------------------------------------------------
            Some(types::DefaultValue::Call) => quote! {
                ::core::option::Option::<#ty>::Some(<#ty as ::core::default::Default>::default())
            },
            // --------------------------------------------------
            // #[klv(..)] <-- field is required
            // --------------------------------------------------
            None => quote! {
                ::core::option::Option::<#ty>::None
            },
        };
        // --------------------------------------------------
        // return
        // --------------------------------------------------
        Some(quote! { #field_name: #init, })
    });

    // --------------------------------------------------
    // construct, both the struct, and the default impl
    // --------------------------------------------------
    quote! {
        #[doc(hidden)]
        #[automatically_derived]
        #[doc = concat!(" In-flight partial packet for [`", stringify!(#name), "`]. Mirror of the struct with every klv field as `Option<T>` so the decode loop can fill it incrementally and resume across `Packet::NeedMore` boundaries via [`tinyklv::Decoder`]")]
        #[derive(::core::fmt::Debug)]
        #vis struct #partial_name #impl_generics #where_clause {
            #(#field_decls)*
            #phantom_field
        }

        #[automatically_derived]
        impl #impl_generics ::core::default::Default for #partial_name #ty_generics #where_clause {
            #[inline(always)]
            fn default() -> Self {
                Self {
                    #(#default_inits)*
                    #phantom_default
                }
            }
        }
    }
}

/// Generates the [`tinyklv::traits::Partial`] impl for the partial-packet struct
///
/// Emits `Partial::finalize(self) -> Result<Xxx, &'static str>`. For each
/// KLV-annotated field one of two things happens:
///
/// * `Option<T>` field - moved into the final struct literal unchanged
/// * required `T` field - a short-circuit guard unwraps `Some(v)` or returns
///   `Err` with a descriptive message naming the missing field
///
/// Fields without a `#[klv(..)]` attribute are filled via
/// `<T as Default>::default()`; the generated code will not compile if their
/// type does not implement [`Default`]
///
/// # Arguments
///
/// * `struct_name` - The main struct ident, used in missing-field error messages
/// * `partial_name` - The partial struct ident the impl is generated for
/// * `generics` - Generic parameters from the original struct definition
/// * `fields` - All fields of the container; non-KLV fields get `Default::default()`
///
/// # Returns
///
/// A [`proc_macro2::TokenStream`] containing the complete `Partial` impl block
///
/// Emission shape (sketch):
///
/// ```rust no_run ignore
/// impl ::tinyklv::traits::Partial for XxxPartialPacket {
///     type Final = Xxx;
///     fn finalize(self) -> Result<Xxx, ContextError> {
///         // one of these per required (non-Option) klv field:
///         let #required_name = match self.#required_name {
///             Some(v) => v,
///             None => return Err(/* rich ctx */),
///         };
///         // one per Option<T> klv field:
///         let #optional_name = self.#optional_name;
///         // ...
///         Ok(Xxx {
///             #klv_field_name,                  // each klv field by name
///             #non_klv_field: <#ty>::default(), // each non-klv field
///         })
///     }
/// }
/// ```
pub(super) fn gen_partial_impl(
    struct_name: &syn::Ident,
    partial_name: &syn::Ident,
    generics: &syn::Generics,
    fields: &Vec<MainField>,
) -> proc_macro2::TokenStream {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    // --------------------------------------------------
    // symbol for `default` keyword in error messages
    // --------------------------------------------------
    let default_symbol = symbol::DEFAULT_VALUE.to_token_stream();
    // --------------------------------------------------
    // for every required (non-Option) klv field, emit a guard statement
    // that pulls from the partial:
    //
    //   let #name = match self.#name {
    //       Some(v) => v,
    //       None => return Err(/* rich error */),
    //   };
    //
    // --------------------------------------------------
    let required_guards = fields
        .iter()
        .filter(|f| f.attrs.is_some())
        .filter(|f| !helpers::is_option(f.ty))
        .map(|field| {
            let MainField { name, .. } = field;
            quote! {
                let #name = match self.#name {
                    Some(v) => v,
                    None => return ::core::result::Result::Err(
                        concat!(
                            "`",
                            stringify!(#struct_name),
                            "::",
                            stringify!(#name),
                            "` is a required value missing from the packet. To prevent this, this field can be set as optional or an `",
                            stringify!(#default_symbol),
                            "` See tinyklv docs for more information.",
                        )
                    ),
                };
            }
        });
    // --------------------------------------------------
    // optional field pass-through: let #name = self.#name
    // --------------------------------------------------
    let optional_passes = fields
        .iter()
        .filter(|f| f.attrs.is_some())
        .filter(|f| helpers::is_option(f.ty))
        .map(|field| {
            let MainField { name, .. } = field;
            quote! {
                let #name = self.#name;
            }
        });
    // --------------------------------------------------
    // klv field names for struct literal (shorthand assignment)
    // rust is awesome
    // --------------------------------------------------
    let klv_field_names = fields
        .iter()
        .filter(|f| f.attrs.is_some())
        .map(|f| f.name.clone());
    // --------------------------------------------------
    // non-klv fields: fill with `#name: <#ty>::default()
    // --------------------------------------------------
    let elem_name_type_without_klv = fields
        .iter()
        .filter_map(|f| match &f.attrs {
            Some(_) => None,
            None => Some((f.name.clone(), f.ty)),
        })
        .collect::<Vec<_>>();
    let default_fields = if elem_name_type_without_klv.is_empty() {
        quote! {}
    } else {
        let names = elem_name_type_without_klv.iter().map(|(n, _)| n.clone());
        let types = elem_name_type_without_klv.iter().map(|(_, ty)| ty);
        quote! { #(#names: <#types as ::core::default::Default>::default(),)* }
    };
    // --------------------------------------------------
    // emit Partial impl
    // --------------------------------------------------
    quote! {
        #[automatically_derived]
        impl #impl_generics ::tinyklv::traits::Partial for #partial_name #ty_generics #where_clause {
            type Final = #struct_name #ty_generics;
            #[inline]
            fn finalize(self) -> ::core::result::Result<
                #struct_name #ty_generics,
                &'static str,
            > {
                #(#required_guards)*
                #(#optional_passes)*
                ::core::result::Result::Ok(#struct_name {
                    #(#klv_field_names,)*
                    #default_fields
                })
            }
        }
    }
}

/// Generates the `TryFrom<XxxPartialPacket> for Xxx` convenience impl
///
/// Emits a `TryFrom` impl with `Error = &'static str` that routes entirely
/// through [`tinyklv::traits::Partial::finalize`], keeping all required-field
/// validation logic in one place. Lets user code write
/// `let x: Xxx = partial.try_into()?;` without needing to import the
/// `Partial` trait
///
/// # Arguments
///
/// * `struct_name` - The target struct ident (the `for Xxx` side)
/// * `partial_name` - The source partial struct ident (the `TryFrom<..>` side)
/// * `generics` - Generic parameters from the original struct definition
///
/// # Returns
///
/// A [`proc_macro2::TokenStream`] containing the complete `TryFrom` impl block
pub(super) fn gen_try_from_partial_impl(
    struct_name: &syn::Ident,
    partial_name: &syn::Ident,
    generics: &syn::Generics,
) -> proc_macro2::TokenStream {
    // --------------------------------------------------
    // generics
    // --------------------------------------------------
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    // --------------------------------------------------
    // emit TryFrom impl routing through Partial::finalize
    // --------------------------------------------------
    quote! {
        #[automatically_derived]
        impl #impl_generics ::core::convert::TryFrom<#partial_name #ty_generics> for #struct_name #ty_generics #where_clause {
            type Error = &'static str;
            #[inline(always)]
            fn try_from(p: #partial_name #ty_generics) -> ::core::result::Result<Self, Self::Error> {
                <#partial_name #ty_generics as ::tinyklv::traits::Partial>::finalize(p)
            }
        }
    }
}
