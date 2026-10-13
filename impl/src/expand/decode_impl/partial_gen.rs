//! Code generation for the partial-packet struct and its trait implementations
//!
//! Provides three generators consumed by [`super::gen_decode_impl`], all methods of
//! [`DecodeCtx`]:
//!
//! * [`DecodeCtx::partial_struct`] - emits the hidden `XxxPartialPacket` struct (a
//!   mirror of the user struct with every KLV field as `Option<T>`, and the `#[klv(unknown)]`
//!   field as its own type), its manual [`Default`] impl seeded by any field-level `default`
//!   attributes, and its [`Debug`] impl, which names the fields without their values
//! * [`DecodeCtx::partial_impl`] - emits the [`tinyklv::traits::Partial`] impl,
//!   which validates required fields and constructs the final struct via
//!   `Partial::finalize`
//! * [`DecodeCtx::try_from_partial_impl`] - emits a `TryFrom<XxxPartialPacket> for
//!   Xxx` convenience impl that routes through `Partial::finalize`
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{DecodeCtx, ErrorKind};
use crate::{
    ast::{
        attr::{MainField, UnknownField},
        types,
    },
    symbol,
};

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::{ToTokens, quote};

/// [`DecodeCtx`] implementation of the partial-packet struct and its impls
impl DecodeCtx<'_> {
    /// Generates the partial-packet struct definition and its manual [`Default`] and [`Debug`]
    /// impls
    ///
    /// The emitted struct mirrors the main struct with one `Option<T>` field per
    /// KLV-annotated field; non-KLV fields are omitted from the partial and filled
    /// via `<T>::default()` only at finalization time. The `#[klv(unknown)]` field keeps its own
    /// type, starting at its type's default, the empty `Vec` the decode loop pushes each unknown
    /// frame into. Generic parameters are forwarded as-is; a `PhantomData` field is added when the
    /// original struct carries type or lifetime parameters so the compiler does not reject the
    /// partial struct for unused generics
    ///
    /// The `Default` impl seeds each field according to its `default` attribute:
    ///
    /// * no `default` attribute  -> `Option::<T>::None`
    /// * bare `default`          -> `Some(<T as ::core::default::Default>::default())`, or on an
    ///   `Option<T>` field its own default, `Option::<T>::None`
    /// * `default = <expr>`      -> `Some(#expr)`
    ///
    /// A manual `Default` impl (rather than `#[derive(Default)]`) is required
    /// because `default = <expr>` may be an arbitrary expression that does not
    /// match `<T as Default>::default()`
    ///
    /// The `Debug` impl prints each field's name with `..` for its value, so a logged partial
    /// shows no decoded data, and asks no field type to be `Debug`
    ///
    /// The partial struct takes the visibility of the original struct
    ///
    /// # Returns
    ///
    /// A [`proc_macro2::TokenStream`] containing the partial struct definition and
    /// its `Default` and `Debug` impl blocks
    pub(super) fn partial_struct(&self) -> proc_macro2::TokenStream {
        let name = &self.input.ident;
        let partial_name = &self.partial_name;
        let partial_ty = &self.partial_ty;
        let vis = &self.input._original.vis;
        let generics = self.input.generics;
        // --------------------------------------------------
        // split generics
        // --------------------------------------------------
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        // --------------------------------------------------
        // Option<T> fields flattened: store Option<T>, not Option<Option<T>>
        // --------------------------------------------------
        let field_decls = self.input.keyed_fields().map(|(field, _)| {
            let field_name = &field.name;
            let ty = field.value_ty();
            // --------------------------------------------------
            // `pub #field_name: Option<#ty>,` per field
            // --------------------------------------------------
            quote! {
                pub #field_name: ::core::option::Option<#ty>,
            }
        });
        // --------------------------------------------------
        // user structs may carry type params that only appear in non-klv fields
        // therefore, must extract type, lifetime, and other params
        // to put them into a phantom data
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
        let (phantom_field, phantom_default) =
            if type_params.is_empty() && lifetime_params.is_empty() {
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
        let default_inits = self.input.keyed_fields().map(|(field, attrs)| {
            let field_name = &field.name;
            let default = attrs.default.clone();
            let ty = field.value_ty();
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
                Some(types::DefaultValue::Call) if !types::is_option(field.ty) => quote! {
                    ::core::option::Option::<#ty>::Some(<#ty as ::core::default::Default>::default())
                },
                // --------------------------------------------------
                // #[klv(..)] <-- field is required, or an `Option` field's default, `None`
                // --------------------------------------------------
                Some(types::DefaultValue::Call) | None => quote! {
                    ::core::option::Option::<#ty>::None
                },
            };
            // --------------------------------------------------
            // return
            // --------------------------------------------------
            quote! { #field_name: #init, }
        });
        // --------------------------------------------------
        // the `unknown` field as its own type, starting at its type's default, which the
        // struct's doc names
        // --------------------------------------------------
        let unknown = self.input.kept();
        let (unknown_decl, unknown_init) = unknown
            .map(|UnknownField { name, ty, .. }| {
                (
                    quote! { pub #name: #ty, },
                    quote! { #name: <#ty as ::core::default::Default>::default(), },
                )
            })
            .unzip();
        let mirror = if unknown.is_some() {
            quote! { "`]. Mirror of the struct with every keyed klv field as `Option<T>` and its `#[klv(unknown)]` field as its own type, so the decode loop can fill it incrementally and resume across `Packet::NeedMore` boundaries via [`tinyklv::Decoder`]" }
        } else {
            quote! { "`]. Mirror of the struct with every klv field as `Option<T>` so the decode loop can fill it incrementally and resume across `Packet::NeedMore` boundaries via [`tinyklv::Decoder`]" }
        };
        // --------------------------------------------------
        // the fields the `Debug` impl names, each with `..` for its value
        // --------------------------------------------------
        let debug_names = self
            .input
            .keyed_fields()
            .map(|(field, _)| &field.name)
            .chain(unknown.map(|UnknownField { name, .. }| name));
        // --------------------------------------------------
        // construct the struct, and its default and debug impls
        // --------------------------------------------------
        quote! {
            #[doc(hidden)]
            #[automatically_derived]
            #[doc = ::core::concat!(" In-flight partial packet for [`", ::core::stringify!(#name), #mirror)]
            #vis struct #partial_name #impl_generics #where_clause {
                #(#field_decls)*
                #unknown_decl
                #phantom_field
            }

            #[automatically_derived]
            impl #impl_generics ::core::default::Default for #partial_ty #where_clause {
                #[inline(always)]
                fn default() -> Self {
                    Self {
                        #(#default_inits)*
                        #unknown_init
                        #phantom_default
                    }
                }
            }

            #[automatically_derived]
            impl #impl_generics ::core::fmt::Debug for #partial_ty #where_clause {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    f.debug_struct(::core::stringify!(#partial_name))
                        #(.field(::core::stringify!(#debug_names), &::core::format_args!("..")))*
                        .finish()
                }
            }
        }
    }

    /// Generates the [`tinyklv::traits::Partial`] impl for the partial-packet struct
    ///
    /// Emits `Partial::finalize(self) -> Result<Xxx, PartialError>`. For each
    /// KLV-annotated field one of two things happens:
    ///
    /// * `Option<T>` field - moved into the final struct literal unchanged
    /// * required `T` field - a short-circuit guard unwraps `Some(v)` or returns
    ///   `Err` with [`ErrorKind::MissingRequired`] and a message naming the missing field
    ///
    /// Fields without a `#[klv(..)]` attribute are filled via
    /// `<T as Default>::default()`; the generated code will not compile if their
    /// type does not implement [`Default`]. The `#[klv(unknown)]` field is moved from the partial
    /// unchanged. A missing required field's error message names the main struct
    ///
    /// Emission shape (sketch):
    ///
    /// ```rust no_run ignore
    /// impl ::tinyklv::traits::Partial for XxxPartialPacket {
    ///     type Final = Xxx;
    ///     fn finalize(self) -> Result<Xxx, PartialError> {
    ///         // one of these per required (non-Option) klv field:
    ///         let #required_name = match self.#required_name {
    ///             Some(v) => v,
    ///             None => return Err(/* `MissingRequired`, naming the missing field */),
    ///         };
    ///         // one per Option<T> klv field:
    ///         let #optional_name = self.#optional_name;
    ///         // ...
    ///         Ok(Xxx {
    ///             #klv_field_name,                  // each klv field by name
    ///             #non_klv_field: <#ty>::default(), // each non-klv field
    ///             #unknown_field: self.#unknown_field,
    ///         })
    ///     }
    /// }
    /// ```
    ///
    /// # Returns
    ///
    /// A [`proc_macro2::TokenStream`] containing the complete `Partial` impl block
    pub(super) fn partial_impl(&self) -> proc_macro2::TokenStream {
        let struct_name = &self.input.ident;
        let partial_ty = &self.partial_ty;
        let (impl_generics, ty_generics, where_clause) = self.input.generics.split_for_impl();
        // --------------------------------------------------
        // symbol for `default` keyword in error messages
        // --------------------------------------------------
        let default_symbol = symbol::DEFAULT_VALUE.to_token_stream();
        let missing = ErrorKind::MissingRequired;
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
        let required_guards = self
            .input
            .keyed_fields()
            .filter(|(f, _)| !types::is_option(f.ty))
            .map(|(field, _)| {
                let MainField { name, .. } = field;
                quote! {
                    let #name = match self.#name {
                        Some(v) => v,
                        None => return ::core::result::Result::Err(
                            ::tinyklv::traits::PartialError::new(&(#missing, ::core::concat!(
                                "`",
                                ::core::stringify!(#struct_name),
                                "::",
                                ::core::stringify!(#name),
                                "` is a required value missing from the packet; to allow this, make the field an `Option` or give it a `",
                                ::core::stringify!(#default_symbol),
                                "`",
                            )))
                        ),
                    };
                }
            });
        // --------------------------------------------------
        // optional field pass-through: let #name = self.#name
        // --------------------------------------------------
        let optional_passes = self
            .input
            .keyed_fields()
            .filter(|(f, _)| types::is_option(f.ty))
            .map(|(field, _)| {
                let MainField { name, .. } = field;
                quote! {
                    let #name = self.#name;
                }
            });
        // --------------------------------------------------
        // klv field names for struct literal (shorthand assignment)
        // rust is awesome
        // --------------------------------------------------
        let klv_field_names = self.input.keyed_fields().map(|(f, _)| &f.name);
        // --------------------------------------------------
        // non-klv fields: fill with `#name: <#ty>::default()
        // --------------------------------------------------
        let default_fields = self.input.plain_fields().map(|MainField { name, ty, .. }| {
            quote! { #name: <#ty as ::core::default::Default>::default(), }
        });
        // --------------------------------------------------
        // the `unknown` field, which the decode loop filled: `#name: self.#name`
        // --------------------------------------------------
        let unknown_field = self
            .input
            .kept()
            .map(|UnknownField { name, .. }| quote! { #name: self.#name, });
        // --------------------------------------------------
        // emit Partial impl
        // --------------------------------------------------
        quote! {
            #[automatically_derived]
            impl #impl_generics ::tinyklv::traits::Partial for #partial_ty #where_clause {
                type Final = #struct_name #ty_generics;
                #[inline]
                fn finalize(self) -> ::core::result::Result<
                    #struct_name #ty_generics,
                    ::tinyklv::traits::PartialError,
                > {
                    #(#required_guards)*
                    #(#optional_passes)*
                    ::core::result::Result::Ok(#struct_name {
                        #(#klv_field_names,)*
                        #(#default_fields)*
                        #unknown_field
                    })
                }
            }
        }
    }

    /// Generates the `TryFrom<XxxPartialPacket> for Xxx` convenience impl
    ///
    /// Emits a `TryFrom` impl with `Error = PartialError` that routes entirely
    /// through [`tinyklv::traits::Partial::finalize`], keeping all required-field
    /// validation logic in one place. Lets user code write
    /// `let x: Xxx = partial.try_into()?;` without needing to import the
    /// `Partial` trait
    ///
    /// # Returns
    ///
    /// A [`proc_macro2::TokenStream`] containing the complete `TryFrom` impl block
    pub(super) fn try_from_partial_impl(&self) -> proc_macro2::TokenStream {
        let struct_name = &self.input.ident;
        let partial_ty = &self.partial_ty;
        // --------------------------------------------------
        // generics
        // --------------------------------------------------
        let (impl_generics, ty_generics, where_clause) = self.input.generics.split_for_impl();
        // --------------------------------------------------
        // emit TryFrom impl routing through Partial::finalize
        // --------------------------------------------------
        quote! {
            #[automatically_derived]
            impl #impl_generics ::core::convert::TryFrom<#partial_ty> for #struct_name #ty_generics #where_clause {
                type Error = ::tinyklv::traits::PartialError;
                #[inline(always)]
                fn try_from(p: #partial_ty) -> ::core::result::Result<Self, Self::Error> {
                    <#partial_ty as ::tinyklv::traits::Partial>::finalize(p)
                }
            }
        }
    }
}
