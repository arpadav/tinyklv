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
use quote::quote;
use quote::ToTokens;

/// Generates the partial struct definition + manual `Default` impl
///
/// The struct mirrors the main struct with one `Option<T>` field per
/// klv-annotated field; non-klv fields are omitted (they are filled via
/// `<#ty>::default()` only at finalisation time). The `Default` impl seeds
/// each field according to its `default` attribute:
///
/// * no attribute       -> `Option::<#ty>::None`
/// * bare `default`     -> `Some(<#ty as ::core::default::Default>::default())`
/// * `default = <expr>` -> `Some(#expr)`
///
/// Manual `Default` (rather than `#[derive(Default)]`) is required because
/// the user's `default = <expr>` may be an arbitrary expression that does
/// not match `<T as Default>::default()`.
///
/// Replaces the historical `gen_items_default` which emitted free local
/// `let mut #name: Option<#ty> = ...;` accumulators inside the
/// `decode_partial` body. Hoisting them onto a named struct lets the
/// state outlive the call so a `Progress::NeedMore(Decoder<P>)` can
/// carry the in-flight partial across `feed`/`next` boundaries.
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
    // user `Option<T>` fields are flattened (storing
    // `Option<T>`, not `Option<Option<T>>`)
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
    // get the phantom field, and for default, same field
    // exists, just without the type params
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
        #[doc = concat!(" In-flight partial packet for [`", stringify!(#name), "`]. Mirror of the struct with every klv field as `Option<T>` so the decode loop can fill it incrementally and resume across `Progress::NeedMore` boundaries via [`tinyklv::Decoder`]")]
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

/// Generates the [`tinyklv::traits::Partial`] impl for the partial-packet
/// struct: `Partial::finalize(self) -> Result<Xxx, ContextError>`.
///
/// For each field that has a `#[klv(..)]` attribute, one of two things happens:
///
/// * `Option<T>` field  -> moved into the struct literal unchanged
/// * required `T` field -> a short-circuit guard is emitted above the struct
///   literal which unwraps `Some(v)` into a plain `v` of type `T`, or returns
///   [`Err(ContextError)`] with a rich, field-naming message if
///   that field was never populated during the decode loop
///
/// Fields that do NOT carry a `#[klv(..)]` attribute are filled via
/// [`Default::default`] inside the struct literal. If any such field's type
/// does not implement [`Default`], the generated code will fail to compile -
/// this is intentional, matching the behavior of the prior result-based
/// `gen_item_set`.
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
///
/// Counterpart of the historical `gen_item_set_progress` which inlined
/// these guards directly into the `decode_partial` body. Hoisting the
/// validation into `Partial::finalize` keeps it reachable from both the
/// end-of-loop path inside `resume_partial` AND the user-facing
/// `TryFrom<Partial> for Xxx` impl, with a single source of truth.
pub(super) fn gen_partial_impl(
    struct_name: &syn::Ident,
    partial_name: &syn::Ident,
    generics: &syn::Generics,
    fields: &Vec<MainField>,
) -> proc_macro2::TokenStream {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    // --------------------------------------------------
    // symbol used to refer to the `default` keyword in error messages
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
    // this simply does `let #name = self.#name` since
    // the original fields in the partial are optional,
    // and so are the ones in the final struct
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
    // assignment on KLV fields: `#name`
    // --------------------------------------------------
    // no need to assign using `#name: #name` since names
    // match and rust is awesome
    // --------------------------------------------------
    let klv_field_names = fields
        .iter()
        .filter(|f| f.attrs.is_some())
        .map(|f| f.name.clone());
    // --------------------------------------------------
    // assignment on NON-klv fields: trailing `#name: <#ty>::default(),`
    // --------------------------------------------------
    // collect fields WITHOUT a `#[klv(..)]` attribute
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
        let types = elem_name_type_without_klv
            .iter()
            .map(|(_, ty)| helpers::type2fish(ty));
        quote! { #(#names: #types::default(),)* }
    };
    // --------------------------------------------------
    // impl partial for the partial struct
    // --------------------------------------------------
    quote! {
        #[automatically_derived]
        impl #impl_generics ::tinyklv::traits::Partial for #partial_name #ty_generics #where_clause {
            type Final = #struct_name #ty_generics;
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

/// Generates the ergonomic `TryFrom<XxxPartialPacket> for Xxx`
/// impl with `Error = ContextError`. Routes through
/// [`tinyklv::traits::Partial::finalize`] so the validation logic lives
/// in exactly one place ([`gen_partial_impl`]).
///
/// Lets user code write `let x: Xxx = partial.try_into()?;` without
/// needing to import the `Partial` trait.
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
    // return the tryfrom
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
