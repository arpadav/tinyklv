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
/// Generates the tokens for the entire [`tinyklv::prelude::DecodeValue`] implementation
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
    let deny_unknown_keys = input.attrs.deny_unknown_keys.is_some();

    let items_default = gen_items_default(&input.data);
    let items_match = gen_items_match(&input.data, &stream, debug);
    let items_set_progress = gen_item_set_progress(name, &input.data);
    let known_keys_pattern = gen_known_keys_pattern(&input.data);

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
                    // ------------- vvv ---- remember this is PACKET_LIFETIME_CHAR
                    fn seek_sentinel<'z>(input: &mut #stream_lifetimed) -> ::tinyklv::__export::winnow::Result<#stream_lifetimed> {
                    // ------------- ^^^ ---- remember this is PACKET_LIFETIME_CHAR
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
    // pre-match unknown-key gate. runs BEFORE `take(len)` so a truncated
    // packet with an unknown key reports "unknown key" instead of
    // "truncated"
    // --------------------------------------------------
    let pre_check_gate = if deny_unknown_keys {
        quote! {
            if !matches!(key, #known_keys_pattern) {
                return ::tinyklv::traits::Progress::Malformed(
                    ::tinyklv::__export::winnow::error::ContextError::new()
                        .add_context(
                            input,
                            &checkpoint_inner,
                            ::tinyklv::__export::winnow::error::StrContext::Label("invalid key"),
                        )
                        .add_context(
                            input,
                            &checkpoint_inner,
                            ::tinyklv::__export::winnow::error::StrContext::Expected(
                                ::tinyklv::__export::winnow::error::StrContextValue::Description(
                                    concat!(
                                        "expected one of the keys defined on `",
                                        stringify!(#name),
                                        "`. To turn this off, remove `deny_unknown_keys`",
                                    )
                                )
                            ),
                        )
                );
            }
        }
    } else {
        quote! {}
    };

    let result = quote! {
        #seek_if_sentinel

        #[doc(hidden)]
        #[automatically_derived]
        #[doc = concat!(" [`", stringify!(#name), "`] implementation of [`tinyklv::prelude::DecodePartial`] for [`", stringify!(#stream), "`]. Source of truth for the decode loop; [`tinyklv::prelude::DecodeValue`] delegates to this.")]
        impl #impl_generics ::tinyklv::traits::DecodePartial<#stream> for #name #ty_generics #where_clause {
            // --------------------------------------------------
            // the generated `matches!(key, K1 | K2 | ...)` gate for
            // `deny_unknown_keys` trips the `manual_range_patterns`
            // lint when the user's keys happen to be consecutive
            // (e.g. 0x01 | 0x02 | 0x03). suppress here so user code
            // does not need a `#[allow]` on every derive target.
            // --------------------------------------------------
            #[allow(clippy::manual_range_patterns)]
            fn decode_partial(
                input: &mut #stream,
            ) -> ::tinyklv::traits::Progress<Self> {
                #items_default
                // --------------------------------------------------
                // `checkpoint` anchors the start of this call for
                // error-context attribution in `items_set_progress`.
                // `checkpoint_inner` is captured per iteration so we
                // can rewind before returning `NeedMore` without
                // re-computing the pre-key/len position.
                // --------------------------------------------------
                let checkpoint = input.checkpoint();
                loop {
                    // --------------------------------------------------
                    // clean EOF check: no bytes left at all.
                    //
                    // this is the normal end-of-packet signal - we
                    // break out of the loop and let `items_set_progress`
                    // assemble whatever has accumulated. required
                    // fields that never arrived become `Malformed`
                    // there.
                    // --------------------------------------------------
                    if input.eof_offset() == 0 {
                        break;
                    }
                    let checkpoint_inner = input.checkpoint();
                    // --------------------------------------------------
                    // parse the next (key, len) pair using the user's
                    // supplied decoders. these decoders are written
                    // against `#stream` directly (typically `&[u8]`),
                    // so they return `winnow::Result<_>` which is
                    // `Result<_, ContextError>` - no `ErrMode` in the
                    // picture.
                    //
                    // a failure here means some bytes existed but did
                    // not form a valid key/len. that's malformed, not
                    // truncated: we rewind to the iteration start so
                    // the caller sees the bad prefix intact and can
                    // choose how to advance past it (e.g. `Decoder`
                    // drops one byte and retries).
                    // --------------------------------------------------
                    let (key, len) = match (
                        #key_decoder,
                        #len_decoder,
                    ).parse_next(input) {
                        Ok(kl) => kl,
                        Err(e) => {
                            input.reset(&checkpoint_inner);
                            return ::tinyklv::traits::Progress::Malformed(
                                e.add_context(
                                    input,
                                    &checkpoint_inner,
                                    ::tinyklv::__export::winnow::error::StrContext::Label(
                                        concat!(
                                            "`",
                                            stringify!(#name),
                                            "` key/len parse failed",
                                        ),
                                    ),
                                )
                            );
                        }
                    };
                    // --------------------------------------------------
                    // break-condition dispatch. `Skip` needs its own
                    // `NeedMore` pre-check for the same reason `take(len)`
                    // below does: we want to rewind cleanly on a short
                    // read, not error out mid-skip.
                    // --------------------------------------------------
                    match Self::break_condition(key, len) {
                        ::tinyklv::BreakConditionType::Proceed => (),
                        ::tinyklv::BreakConditionType::Skip => {
                            if input.eof_offset() < len {
                                let short = len - input.eof_offset();
                                input.reset(&checkpoint_inner);
                                return ::tinyklv::traits::Progress::NeedMore(
                                    ::tinyklv::__export::winnow::stream::Needed::new(short)
                                );
                            }
                            // pre-checked above; `take` cannot fail here
                            let _ = ::tinyklv::__export::winnow::token::take::<
                                usize,
                                #stream,
                                ::tinyklv::__export::winnow::error::ContextError,
                            >(len).parse_next(input);
                            continue;
                        }
                        ::tinyklv::BreakConditionType::Done => break,
                        ::tinyklv::BreakConditionType::Abort(e) => {
                            return ::tinyklv::traits::Progress::Malformed(e);
                        }
                    }
                    #debug_key_val
                    // --------------------------------------------------
                    // unknown-key gate BEFORE `take(len)`. emitted only
                    // when `deny_unknown_keys` is set on the container
                    // --------------------------------------------------
                    #pre_check_gate
                    // --------------------------------------------------
                    // `NeedMore` pre-check for the value take. if the
                    // declared length overruns remaining input, rewind
                    // the cursor to the start of this iteration so the
                    // caller can retry after feeding more bytes, and
                    // hand back a precise `Needed::Size(delta)`.
                    // --------------------------------------------------
                    if input.eof_offset() < len {
                        let short = len - input.eof_offset();
                        input.reset(&checkpoint_inner);
                        return ::tinyklv::traits::Progress::NeedMore(
                            ::tinyklv::__export::winnow::stream::Needed::new(short)
                        );
                    }
                    // --------------------------------------------------
                    // extract the value subinput. pre-checked above, so
                    // the take cannot fail in practice; on the unlikely
                    // parser-internal error we still surface it cleanly
                    // as `Malformed` rather than panicking.
                    // --------------------------------------------------
                    let mut subinput: <#stream as ::tinyklv::__export::winnow::stream::Stream>::Slice = match ::tinyklv::__export::winnow::token::take::<
                        usize,
                        #stream,
                        ::tinyklv::__export::winnow::error::ContextError,
                    >(len).parse_next(input) {
                        Ok(s) => s,
                        Err(e) => return ::tinyklv::traits::Progress::Malformed(e),
                    };
                    // --------------------------------------------------
                    // field dispatch. unknown keys (when allowed) hit
                    // the `_ => ()` arm and are silently ignored after
                    // their bytes have already been consumed by the
                    // take above.
                    // --------------------------------------------------
                    match key {
                        #items_match
                        _ => (),
                    }
                }
                #items_set_progress
            }
        }

        #[doc(hidden)]
        #[automatically_derived]
        #[doc = concat!(" [`", stringify!(#name), "`] implementation of [`tinyklv::prelude::DecodeValue`] for [`", stringify!(#stream), "`]. Thin delegating wrapper around [`tinyklv::prelude::DecodePartial`]: the streaming loop is the single source of truth, and the one-shot contract is expressed by mapping [`tinyklv::prelude::Progress`] to [`tinyklv::__export::winnow::Result`]. `NeedMore` becomes a truncation error here because the `decode_value` caller has already committed all its bytes.")]
        impl #impl_generics ::tinyklv::traits::DecodeValue<#stream> for #name #ty_generics #where_clause {
            fn decode_value(input: &mut #stream) -> ::tinyklv::__export::winnow::Result<Self> {
                // --------------------------------------------------
                // take the outer checkpoint for the truncation error
                // context. both impls run on the SAME `#stream` type,
                // so the checkpoint type matches everywhere and no
                // cross-stream coercion is needed.
                // --------------------------------------------------
                let checkpoint = input.checkpoint();
                match <Self as ::tinyklv::traits::DecodePartial<#stream>>::decode_partial(input) {
                    ::tinyklv::traits::Progress::Ready(v) => Ok(v),
                    ::tinyklv::traits::Progress::Malformed(e) => Err(e),
                    ::tinyklv::traits::Progress::NeedMore(_) => Err(
                        ::tinyklv::__export::winnow::error::ContextError::new()
                            .add_context(
                                input,
                                &checkpoint,
                                ::tinyklv::__export::winnow::error::StrContext::Label(
                                    concat!(
                                        "`",
                                        stringify!(#name),
                                        "` packet truncated: one-shot `decode_value` ran out of input",
                                    ),
                                ),
                            )
                    ),
                }
            }
        }
    };
    result
}

/// Generates the tokens for initializing the field variables as optional
///
/// Emission depends on the field's `default` attribute:
///
/// * no attribute       -> `let mut #name: Option<#ty> = None;`
/// * bare `default`     -> `let mut #name: Option<#ty> = Some(<#ty as ::core::default::Default>::default());`
/// * `default = <expr>` -> `let mut #name: Option<#ty> = Some(#expr);`
fn gen_items_default(fatts: &Vec<MainField>) -> proc_macro2::TokenStream {
    let field_initializations = fatts.iter().map(|field| {
        let MainField { name, ty, .. } = field;
        let default = field.attrs.as_ref().and_then(|f| f.default.clone());
        let ty = helpers::unwrap_option_type(ty).unwrap_or(ty);
        match default {
            Some(types::DefaultValue::Expr(expr)) => quote! {
                let mut #name: Option<#ty> = Some(#expr);
            },
            Some(types::DefaultValue::Call) => quote! {
                let mut #name: Option<#ty> =
                    Some(<#ty as ::core::default::Default>::default());
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
fn gen_items_match(
    fields: &Vec<MainField>,
    stream: &syn::Type,
    debug: bool,
) -> proc_macro2::TokenStream {
    let arms = fields
        .iter()
        .filter_map(|f| f.attrs.as_ref().map(|attr| (&f.name, f.ty, attr)))
        .map(|(name, ty, attrs)| {
            // --------------------------------------------------
            // the name of the field assigned above.
            // this is a variable which is assigned Option<T>
            // --------------------------------------------------
            // the key which represents the field in binary
            // --------------------------------------------------
            let key = &attrs.key;
            // --------------------------------------------------
            // the value decoder
            //
            // when `fallback_dec` is set, re-synthesize a fully-qualified
            // `<T as ::tinyklv::traits::DecodeValue<#stream>>::decode_value`
            // path so rustc emits a clean trait-bound error if the trait is
            // not implemented for the field's type
            // --------------------------------------------------
            let dec_tokens: proc_macro2::TokenStream = if attrs.fallback_dec {
                let t = helpers::unwrap_option_type(ty).unwrap_or(ty);
                quote! { <#t as ::tinyklv::traits::DecodeValue<#stream>>::decode_value }
            } else {
                #[allow(clippy::unwrap_used, reason = "`gen_decode_impl` call ensures that `attrs.dec` is `Some`")]
                let dec = attrs.dec.as_ref().unwrap();
                quote! { #dec }
            };
            let varlen = attrs.var.as_ref().map(|v| v.value).unwrap_or(false); // <-- defaults to false
            let optional_len_arg = if varlen {
                quote! { (len) }
            } else {
                quote! {}
            };
            // --------------------------------------------------
            // post-decode latebind
            // --------------------------------------------------
            // consuming (`is_mut == false`): `.map(inner)` - `Fn(T) -> U`
            // mutating  (`is_mut == true`):  `.map(|mut __v| { inner(&mut __v); __v })` - `Fn(&mut T)`
            // --------------------------------------------------
            let latebind_map = match attrs.latebind.as_ref() {
                Some(lb) => {
                    let inner = &lb.inner;
                    if lb.is_mut {
                        quote! { .map(|mut __v| { #inner(&mut __v); __v }) }
                    } else {
                        quote! { .map(#inner) }
                    }
                }
                None => quote! {},
            };
            // --------------------------------------------------
            // return
            // --------------------------------------------------
            match debug {
                true => {
                    let logger = logger();
                    quote! {
                        #key => {
                            let val = #dec_tokens #optional_len_arg (&mut subinput);
                            #logger ("\t{}: {:?}", stringify!(#name), val);
                            #name = val.ok() #latebind_map .or(#name);
                        },
                    }
                }
                false => quote! {
                    #key => #name = #dec_tokens #optional_len_arg (&mut subinput).ok() #latebind_map .or(#name),
                },
            }
        });
    // --------------------------------------------------
    // return all the match arms
    // --------------------------------------------------
    quote! { #(#arms)* }
}

/// Generates the final-return tokens for the [`tinyklv::prelude::DecodePartial`]
/// impl's `decode_partial` body.
///
/// For each field that has a `#[klv(..)]` attribute, one of two things happens:
///
/// * `Option<T>` field  -> moved into the struct literal unchanged
/// * required `T` field -> a short-circuit guard is emitted above the struct
///   literal which unwraps `Some(v)` into a plain `v` of type `T`, or returns
///   [`tinyklv::prelude::Progress::Malformed`] with a rich context error if
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
/// // one of these per required (non-Option) klv field:
/// let #required_name = match #required_name {
///     Some(v) => v,
///     None => return ::tinyklv::traits::Progress::Malformed(/* rich ctx */),
/// };
/// // ...
/// ::tinyklv::traits::Progress::Ready(#struct_name {
///     // every klv field (required + optional) by name:
///     #klv_field_name,
///     // every non-klv field, defaulted:
///     #non_klv_field: <#ty>::default(),
/// })
/// ```
///
/// Counterpart of the historical `gen_item_set` which emitted the Result
/// flavor. [`tinyklv::prelude::DecodeValue`] now delegates to
/// [`tinyklv::prelude::DecodePartial`] and maps `Progress -> Result` at the
/// boundary, so only the `Progress` emission lives here.
fn gen_item_set_progress(
    struct_name: &syn::Ident,
    fields: &Vec<MainField>,
) -> proc_macro2::TokenStream {
    // --------------------------------------------------
    // symbol used to refer to the `default` keyword in error messages
    // --------------------------------------------------
    let default_symbol = symbol::DEFAULT_VALUE.to_token_stream();
    // --------------------------------------------------
    // collect fields WITHOUT a `#[klv(..)]` attribute. they will be
    // filled via `<#ty>::default()` in the struct literal below. if any
    // such field's type lacks a `Default` impl, the generated code will
    // fail to compile - intentional, forces the user to either annotate
    // the field or implement `Default`
    // --------------------------------------------------
    let elem_name_type_without_klv = fields
        .iter()
        .filter_map(|f| match &f.attrs {
            Some(_) => None,
            None => Some((f.name.clone(), f.ty)),
        })
        .collect::<Vec<_>>();
    // --------------------------------------------------
    // for every required (non-Option) klv field, emit a guard statement
    // that unwraps its accumulator:
    //
    //   let #name = match #name {
    //       Some(v) => v,
    //       None => return Progress::Malformed(/* rich error */),
    //   };
    //
    // this relies on the accumulator being a `let mut #name: Option<T>`
    // as emitted by `gen_items_default`. after this guard, `#name` is
    // shadowed as a plain `T` so the struct literal can move it in
    // --------------------------------------------------
    let required_guards = fields
        .iter()
        .filter(|f| f.attrs.is_some())
        .filter(|f| !helpers::is_option(f.ty))
        .map(|field| {
            let MainField { name, .. } = field;
            quote! {
                let #name = match #name {
                    Some(v) => v,
                    None => return ::tinyklv::traits::Progress::Malformed(
                        ::tinyklv::__export::winnow::error::ContextError::new().add_context(
                            input,
                            &checkpoint,
                            ::tinyklv::__export::winnow::error::StrContext::Label(
                                concat!(
                                    "`",
                                    stringify!(#struct_name),
                                    "::",
                                    stringify!(#name),
                                    "` is a required value missing from the packet. To prevent this, this field can be set as optional or an `",
                                    stringify!(#default_symbol),
                                    "`.",
                                )
                            )
                        )
                    ),
                };
            }
        });
    // --------------------------------------------------
    // every klv-annotated field contributes its bare name to the struct
    // literal. for required fields, the guard above shadowed the
    // `Option<T>` with a plain `T` of the same ident, so a bare
    // `#name,` shorthand works for both cases
    // --------------------------------------------------
    let klv_field_names = fields
        .iter()
        .filter(|f| f.attrs.is_some())
        .map(|f| f.name.clone());
    // --------------------------------------------------
    // trailing `#name: <#ty>::default(),` block for non-klv fields.
    // empty when there are none, so the struct literal stays tidy
    // --------------------------------------------------
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
    // splice: guards first, then the ready-wrapped struct literal
    // --------------------------------------------------
    quote! {
        #(#required_guards)*
        ::tinyklv::traits::Progress::Ready(#struct_name {
            #(#klv_field_names,)*
            #default_fields
        })
    }
}

/// Generates a `|`-joined pattern of every field's `#[klv(key = ..)]` literal,
/// suitable for the RHS of a `matches!(key, #pattern)` expression.
///
/// Used by the pre-match unknown-key gate inside `decode_partial`: before
/// `take(len)` consumes the value bytes, we check whether `key` is one the
/// struct declared. If not, and the container carries `deny_unknown_keys`, we
/// bail with [`tinyklv::prelude::Progress::Malformed`] immediately - no bytes
/// wasted, and the error says "unknown key" rather than the downstream
/// "packet truncated" the old ordering would have produced when the declared
/// length overran remaining input.
///
/// # Degenerate case
///
/// A struct with zero klv-annotated fields has no known keys at all. We emit
/// `_ if false` which is a never-match pattern, so `matches!(key, _ if false)`
/// is always `false` and every key is treated as unknown. Under
/// `deny_unknown_keys` this rejects everything, which is the only sensible
/// behavior for a struct that declared no keys.
///
/// # Emission shape
///
/// ```text
/// 0x01 | 0x02 | 0x03
/// ```
///
/// or for enum-like keys:
///
/// ```text
/// Key::Foo | Key::Bar
/// ```
fn gen_known_keys_pattern(fields: &Vec<MainField>) -> proc_macro2::TokenStream {
    // --------------------------------------------------
    // collect each klv field's declared key expression
    // --------------------------------------------------
    let keys: Vec<_> = fields
        .iter()
        .filter_map(|f| f.attrs.as_ref().map(|a| a.key.clone()))
        .collect();
    // --------------------------------------------------
    // degenerate: no klv fields -> never-match pattern
    // --------------------------------------------------
    if keys.is_empty() {
        return quote! { _ if false };
    }
    // --------------------------------------------------
    // splice keys with `|` separators to form a pattern alternation
    // --------------------------------------------------
    quote! { #(#keys)|* }
}
