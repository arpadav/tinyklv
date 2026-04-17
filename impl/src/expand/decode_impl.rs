use crate::{
    ast::{
        attr::{MainContainer, MainField},
        types,
    },
    expand::helpers,
    symbol,
};

use quote::quote;
use quote::ToTokens;

const PACKET_LIFETIME_CHAR: char = 'z';

fn logger() -> proc_macro2::TokenStream {
    #[cfg(feature = "tracing")]
    {
        quote! { ::tracing::debug! }
    }
    #[cfg(not(feature = "tracing"))]
    {
        quote! { ::std::println! }
    }
}
/// Generates the tokens for the entire [`tinyklv::prelude::DecodeValue`](https://docs.rs/tinyklv/latest/tinyklv/prelude/trait.DecodeValue.html) implementation
pub(crate) fn gen_decode_impl(
    input: &MainContainer,
    key_decoder: &types::XcoderType,
    len_decoder: &types::XcoderType,
) -> proc_macro2::TokenStream {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    // --------------------------------------------------
    // default stream -> &[u8]
    // --------------------------------------------------
    let stream = input.attrs.stream.clone().unwrap_or(helpers::u8_slice());
    let stream_lifetimed = helpers::insert_lifetime(&stream, PACKET_LIFETIME_CHAR);
    let sentinel = input.attrs.sentinel.as_ref();

    let debug = input.attrs.debug.is_some();
    let deny_unknown_keys = input.attrs._deny_unknown_keys.is_some();

    let items_init = gen_items_init(&input.data);
    let items_match = gen_items_match(&input.data, debug);
    let items_set = gen_item_set(name, &input.data);

    let seek_if_sentinel = match sentinel {
        Some(sentinel) => {
            // let packet_lifetime = quote::format_ident!("'{}", PACKET_LIFETIME_CHAR);
            let sentinel_len_static_name =
                quote::format_ident!("__TINYKLV_SENTINEL_LEN_{}", name.to_string().to_uppercase());
            let sentinel_seeker_static_name =
                quote::format_ident!("__TINYKLV_SEEKER_{}", name.to_string().to_uppercase());
            quote! {
                #[automatically_derived]
                #[doc(hidden)]
                #[doc = concat!(" Static sentinel length for [`", stringify!(#name), "`] implementation of [`tinyklv::prelude::SeekSentinel`]")]
                pub(crate) const #sentinel_len_static_name: usize = #sentinel.len();

                #[automatically_derived]
                #[doc(hidden)]
                #[doc = concat!(" Static seeker for [`", stringify!(#name), "`] implementation of [`tinyklv::prelude::SeekSentinel`]")]
                pub(crate) static #sentinel_seeker_static_name: ::std::sync::LazyLock<::tinyklv::__export::memchr::memmem::Finder> =
                    ::std::sync::LazyLock::new(|| ::tinyklv::__export::memchr::memmem::Finder::new(#sentinel));

                #[automatically_derived]
                #[doc(hidden)]
                #[doc = concat!(" [`", stringify!(#name), "`] implementation of [`tinyklv::prelude::SeekSentinel`] for [`", stringify!(#stream), "`]")]
                impl #impl_generics ::tinyklv::traits::SeekSentinel<#stream> for #name #ty_generics #where_clause {
                    // ---- vvv ---- remember this is PACKET_LIFETIME_CHAR
                    fn seek_sentinel<'z>(input: &mut #stream_lifetimed) -> ::tinyklv::__export::winnow::Result<#stream_lifetimed> {
                    // ---- ^^^ ---- remember this is PACKET_LIFETIME_CHAR
                        let checkpoint = input.checkpoint();
                        match #sentinel_seeker_static_name.find(&input) {
                            Some(position) => *input = &input[position + #sentinel_len_static_name..],
                            None => return Err(
                                ::tinyklv::__export::winnow::error::ContextError::new().add_context(
                                    input,
                                    &checkpoint,
                                    ::tinyklv::__export::winnow::error::StrContext::Label(
                                        concat!("Unable to find recognition sentinel for `", stringify!(#name), "` packet")
                                    ),
                                )
                            )
                        };
                        let checkpoint = input.checkpoint();
                        let packet_len = match #len_decoder.parse_next(input) {
                            Ok(x) => x as usize,
                            Err(e) => return Err(
                                e.add_context(
                                    input,
                                    &checkpoint,
                                    ::tinyklv::__export::winnow::error::StrContext::Label(
                                        concat!("Unable to parse packet length for `", stringify!(#name), "` packet")
                                    ),
                                )
                            ),
                        };
                        ::tinyklv::__export::winnow::token::take(packet_len).parse_next(input)
                    }
                }
            }
        }
        None => quote! {},
    };

    // --------------------------------------------------
    // debug statement after key/len parse
    // --------------------------------------------------
    let debug_key_val = match debug {
        true => {
            let logger = logger();
            quote! {
                #logger ("key: {}, len: {}", key, len);
            }
        }
        false => quote! {},
    };

    // --------------------------------------------------
    // ignore unknown keys, or return error if "deny unknown keys"
    // is set by the user
    // --------------------------------------------------
    let remaining_match = match deny_unknown_keys {
        true => quote! {
            _unknown_key => return Err(
                ::tinyklv::__export::winnow::error::ContextError::new()
                    .add_context(
                        input,
                        &checkpoint,
                        ::tinyklv::__export::winnow::error::StrContext::Label("invalid key"),
                    )
                    .add_context(
                        input,
                        &checkpoint,
                        ::tinyklv::__export::winnow::error::StrContext::Expected(
                            ::tinyklv::__export::winnow::error::StrContextValue::Description(
                                concat!("expected one of the keys defined on `", stringify!(#name), "`. To turn this off, remove `deny_unknown_keys`")
                            )
                        ),
                    )
            ),
        },
        false => quote! {
            _ => (),
        },
    };

    let result = quote! {
        #seek_if_sentinel

        #[doc(hidden)]
        #[automatically_derived]
        #[doc = concat!(" [`", stringify!(#name), "`] implementation of [`tinyklv::prelude::DecodeValue`] for [`", stringify!(#stream), "`]")]
        impl #impl_generics ::tinyklv::traits::DecodeValue<#stream> for #name #ty_generics #where_clause {
            fn decode_value(input: &mut #stream) -> ::tinyklv::__export::winnow::Result<Self> {
                #items_init
                let checkpoint = input.checkpoint();
                loop {
                    let checkpoint_inner = input.checkpoint();
                    match (
                        #key_decoder,
                        #len_decoder,
                    ).parse_next(input) {
                        Ok((key, len)) => {
                            match Self::break_condition(key, len) {
                                ::tinyklv::BreakConditionType::Proceed => (),
                                ::tinyklv::BreakConditionType::Skip => {
                                    let Ok(_) = ::tinyklv::__export::winnow::token::take::<
                                        usize,
                                        #stream,
                                        ::tinyklv::__export::winnow::error::ContextError,
                                    >(len).parse_next(input) else {
                                        break
                                    };
                                    continue;
                                },
                                ::tinyklv::BreakConditionType::Done => break,
                                ::tinyklv::BreakConditionType::Abort(e) => return Err(e),
                            }
                            #debug_key_val
                            let mut subinput = match ::tinyklv::__export::winnow::token::take::<
                                usize,
                                #stream,
                                ::tinyklv::__export::winnow::error::ContextError,
                            >(len).parse_next(input) {
                                Ok(s) => s,
                                // Short-read after a valid key/len: the declared length
                                // overruns the remaining bytes, which means the packet is
                                // truncated mid-value. Bail loudly so the caller sees where
                                // the stream ended unexpectedly.
                                Err(e) => {
                                    return Err(e.add_context(
                                        input,
                                        &checkpoint_inner,
                                        ::tinyklv::__export::winnow::error::StrContext::Label(
                                            concat!(
                                                "`",
                                                stringify!(#name),
                                                "` packet truncated: declared length exceeds remaining input",
                                            ),
                                        ),
                                    ));
                                }
                            };
                            match key {
                                #items_match
                                #remaining_match
                            }
                        },
                        // likely no more input, return what we have
                        // error is thrown away here, could debug print it?
                        Err(_) => break,
                    }
                }
                #items_set
            }
        }
    };
    result
}

/// Generates the tokens for initializing the field variables as optional
///
/// `let mut #name: Option<#ty> = None;`
fn gen_items_init(fatts: &Vec<MainField>) -> proc_macro2::TokenStream {
    let field_initializations = fatts.iter().map(|field| {
        let MainField { name, ty, .. } = field;
        let init = field.attrs.as_ref().and_then(|f| f._init.clone());
        let ty = helpers::unwrap_option_type(ty).unwrap_or(ty);
        match init {
            Some(init) => quote! {
                let mut #name: Option<#ty> = Some(#init);
            },
            None => quote! {
                let mut #name: Option<#ty> = None;
            },
        }
    });
    quote! { #(#field_initializations)* }
}

/// Generates the tokens for matching the key/len's with fields and parsers
///
/// `#key => #name = #dec #optional_len_arg (&mut subinput).ok(),`
///
/// Where `subinput` is a sub-slice of `input` of the values length, designated
/// by the stream after its key.
fn gen_items_match(fields: &Vec<MainField>, debug: bool) -> proc_macro2::TokenStream {
    let arms = fields
        .iter()
        .filter_map(|f| f.attrs.as_ref().map(|attr| (&f.name, attr)))
        .map(|(name, attrs)| {
            // --------------------------------------------------
            // the name of the field assigned above.
            // this is a variable which is assigned Option<T>
            // --------------------------------------------------
            // the key which represents the field in binary
            // --------------------------------------------------
            let key = &attrs.key;
            // --------------------------------------------------
            // the value decoder
            // --------------------------------------------------
            #[allow(clippy::unwrap_used)]
            // `gen_decode_impl` call ensures that `attrs.dec` is `Some`
            let dec = attrs.dec.as_ref().unwrap();
            let varlen = attrs.var.as_ref().map(|v| v.value).unwrap_or(false); // <-- defaults to false
            let optional_len_arg = if varlen {
                quote! { (len) }
            } else {
                quote! {}
            };
            // --------------------------------------------------
            // return
            // --------------------------------------------------
            match debug {
                true => {
                    let logger = logger();
                    quote! {
                        #key => {
                            let val = #dec #optional_len_arg (&mut subinput);
                            #logger ("\t{}: {:?}", stringify!(#name), val);
                            #name = val.ok().or(#name);
                        },
                    }
                }
                false => quote! {
                    #key => #name = #dec #optional_len_arg (&mut subinput).ok().or(#name),
                },
            }
        });
    // --------------------------------------------------
    // return all the match arms
    // --------------------------------------------------
    quote! { #(#arms)* }
}

/// Generates the tokens for setting the field variables upon returning of the output struct
///
/// `Ok(#struct_name { #(#field_set_on_return)* })`
fn gen_item_set(struct_name: &syn::Ident, fields: &Vec<MainField>) -> proc_macro2::TokenStream {
    let init_symbol = symbol::INITIAL_VALUE.to_token_stream();
    let elem_name_type_without_klv = fields
        .iter()
        .filter_map(|f| match &f.attrs {
            Some(_) => None,
            None => Some((f.name.clone(), f.ty)),
        })
        .collect::<Vec<_>>();
    let field_set_on_return = fields.iter().filter_map(|f| f.attrs.as_ref().map(|_| f)).map(|field| {
        let MainField { name, ty, .. } = field;
        match helpers::is_option(ty) {
            false => quote! {
                #name: #name.ok_or(::tinyklv::__export::winnow::error::ContextError::new().add_context(
                        input,
                        &checkpoint,
                        ::tinyklv::__export::winnow::error::StrContext::Label(
                            concat!(
                                "`",
                                stringify!(#struct_name),
                                "::",
                                stringify!(#name),
                                "` is a required value missing from the packet. To prevent this, this field can be set as optional or an `",
                                stringify!(#init_symbol),
                                "`.",
                            )
                        )
                    )
                )?,
            },
            true => quote! { #name, },
        }
    });
    // --------------------------------------------------
    // elements without the  `#[klv(..)]` attribute must
    // implement [`default::Default`]
    // --------------------------------------------------
    // if the default does not exist, then this will not compile
    // --------------------------------------------------
    match !elem_name_type_without_klv.is_empty() {
        false => quote! { Ok(#struct_name { #(#field_set_on_return)* }) },
        true => {
            let names: Vec<_> = elem_name_type_without_klv
                .iter()
                .map(|(name, _)| name.clone())
                .collect();
            let types: Vec<_> = elem_name_type_without_klv
                .iter()
                .map(|(_, ty)| helpers::type2fish(ty))
                .collect();
            let individual_defaults = quote! { #(#names: #types::default(),)* };
            quote! {
                Ok(#struct_name {
                    #(#field_set_on_return)*
                    #individual_defaults
                })
            }
        }
    }
}
