//! Token generation for the `tinyklv::prelude::EncodeFrame`
//! and `tinyklv::prelude::EncodeValue` derive implementations
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::ast::attr::MainContainer;
use crate::ast::types::{self, XcoderSigil};
use crate::expand::helpers;

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::{quote, quote_spanned};

/// Generates the tokens for the entire `tinyklv::prelude::EncodeFrame`
/// and `tinyklv::prelude::EncodeValue` implementations for a container
///
/// Emits an `EncodeValue` impl for every container. If a sentinel is present,
/// also emits a full `EncodeFrame` impl that wraps the value encoding in a KLV
/// packet (key + length + value)
///
/// # Arguments
///
/// * `input` - The parsed container descriptor including field metadata
/// * `key_encoder` - Token expression used to encode each field's key
/// * `len_encoder` - Token expression used to encode the packet length
pub(crate) fn gen_encode_impl(
    input: &MainContainer,
    key_encoder: &types::SiguledXcoder,
    len_encoder: &types::SiguledXcoder,
) -> proc_macro2::TokenStream {
    // --------------------------------------------------
    // extract the container name and sentinel, if any
    // --------------------------------------------------
    let name = &input.ident;
    let sentinel = input.attrs.sentinel.as_ref();
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    // --------------------------------------------------
    // generate the per-field encoding token stream
    // --------------------------------------------------
    let items_encoded = gen_items_encoded(input, key_encoder, len_encoder);
    // --------------------------------------------------
    // compile-time capacity lower bound (sum of fixed-width value bytes plus a small
    // per-field key+length allowance); a hint only, so an under/over estimate is harmless
    // --------------------------------------------------
    let reserve = gen_reserve_hint(input);
    // --------------------------------------------------
    // the per-field value-staging buffer is only needed when the length encoder is variable-width
    // (e.g. BER): then a field's encoded byte length must be known before its length prefix is
    // written, so the value is staged in `__scratch` first. with a fixed-width length encoder every
    // field writes straight into `out` (direct-write or back-patch), so the buffer is omitted
    // --------------------------------------------------
    let scratch_decl = if fixed_len_width(len_encoder).is_none() && has_attributed_fields(input) {
        quote! { let mut __scratch: Vec<u8> = Vec::new(); }
    } else {
        quote! {}
    };
    let encode_with_key_len = match sentinel {
        Some(sentinel) => {
            // --------------------------------------------------
            // frame body. when the length encoder is fixed-width, reserve the length slot up
            // front and write sentinel + slot + value body directly into `out`, then back-patch
            // the length in place - no second body buffer and no full-body copy. for a
            // variable-width length (e.g. BER) the slot width is unknown up front, so fall back
            // to staging the body in a local buffer to learn its byte length first
            // --------------------------------------------------
            let frame_body = if let Some(width) = fixed_len_width(len_encoder) {
                // fixed-width length: write sentinel, then the value body, then back-patch the
                // length slot - same mechanism as a fixed-width-length field (see emit_len_backpatch)
                emit_len_backpatch(
                    quote! {
                        out.reserve(#reserve + #width + #sentinel.len());
                        out.extend_from_slice(#sentinel);
                    },
                    len_encoder,
                    width,
                    quote! { self.encode_value(out); },
                )
            } else {
                // variable-width length (BER): stage the body to learn its byte length first
                let ber_allowance = BER_LEN_PREFIX_ALLOWANCE;
                quote! {
                    let mut __body: Vec<u8> = Vec::new();
                    self.encode_value(&mut __body);
                    out.reserve(__body.len() + #sentinel.len() + #ber_allowance);
                    out.extend_from_slice(#sentinel);
                    #len_encoder(__body.len(), out);
                    out.extend_from_slice(&__body);
                }
            };
            quote! {
                #[automatically_derived]
                impl #impl_generics ::tinyklv::traits::EncodeFrame for #name #ty_generics #where_clause {
                    fn encode_frame(&self, out: &mut Vec<u8>) {
                        #frame_body
                    }
                }
            }
        }
        None => quote! {},
    };
    quote! {
        #[automatically_derived]
        impl #impl_generics ::tinyklv::traits::EncodeValue for #name #ty_generics #where_clause {
            fn encode_value(&self, out: &mut Vec<u8>) {
                out.reserve(#reserve);
                #scratch_decl
                #items_encoded
            }
        }
        #encode_with_key_len
    }
}

/// Generates the token stream that encodes each field of a container into
/// the output byte buffer
///
/// Iterates over fields that have a `#[klv(..)]` attribute and emits one
/// `output.extend(...)` expression per field. Optional fields are wrapped
/// in an `if let Some(ref __val)` guard so they are skipped when absent
///
/// # Arguments
///
/// * `input` - The parsed container descriptor including field metadata
/// * `key_encoder` - Token expression used to encode each field's key
/// * `len_encoder` - Token expression used to encode the value length
///
/// # Safety
///
/// Uses `.unwrap()` on `attrs.enc` - this is safe because `gen_encode_impl`
/// only calls this function when `all_encoders_exist` is `true`, guaranteeing
/// every field attribute carries a non-`None` encoder
fn gen_items_encoded(
    input: &MainContainer,
    key_encoder: &types::SiguledXcoder,
    len_encoder: &types::SiguledXcoder,
) -> proc_macro2::TokenStream {
    // --------------------------------------------------
    // the length-encoder width is the same for every field; computing it once decides the
    // container-wide strategy (fixed-width len -> direct/back-patch; variable-width len -> scratch)
    // --------------------------------------------------
    let len_width = fixed_len_width(len_encoder);
    // --------------------------------------------------
    // map each attributed field to its encoding expression
    // --------------------------------------------------
    let items_encoded = input
        .data
        .iter()
        .filter_map(|field| {
            field
                .attrs
                .as_ref()
                .map(|attr| (&field.name, &field.ty, attr))
        })
        .map(|(name, ty, attrs)| {
            #[allow(
                clippy::unwrap_used,
                reason = "`gen_encode_impl` call ensures that `attrs.enc` is `Some`"
            )]
            let value_encoder = attrs.enc.as_ref().unwrap();
            let key = &attrs.key;
            let span = name.span();
            // --------------------------------------------------
            // when `fallback_enc` is set, the inner path stored on the
            // encoder is an unused placeholder. emit a fully-qualified
            // `<T as EncodeValue>::encode_value` instead so rustc
            // surfaces a clean trait-bound error if the trait is not impl'd
            // --------------------------------------------------
            let enc_tokens: proc_macro2::TokenStream = if attrs.fallback_enc {
                let t = helpers::unwrap_option_type(ty).unwrap_or(ty);
                quote! { <#t as ::tinyklv::traits::EncodeValue>::encode_value }
            } else {
                let inner = &value_encoder.inner;
                quote! { #inner }
            };
            // --------------------------------------------------
            // per-sigil argument shaping:
            //
            // * `None` -> `func(&self.field)` - fn takes `&T`
            //   (deref coercion handles `&String -> &str`, `&Vec<u8> -> &[u8]`, etc.)
            // * `Ref` -> `func(EncodeAs::encode_as(&self.field))` - trait dispatches:
            //   primitives by value (Copy), `String -> &str`, `Vec<T> -> &[T]`,
            //   `Box<T>/Rc<T>/Arc<T> -> &T`. No clone, no heap allocation.
            // * `Deref` -> copy by value
            //
            // Optional (`Option<T>`) mirrors these by operating on `__val: &T`
            // --------------------------------------------------
            let (nonopt_arg, opt_arg) = match value_encoder.sigil {
                XcoderSigil::None => (
                    quote_spanned! { span => &self.#name },
                    quote_spanned! { span => __val },
                ),
                XcoderSigil::Ref => (
                    quote_spanned! { span => ::tinyklv::traits::EncodeAs::encode_as(&self.#name) },
                    quote_spanned! { span => ::tinyklv::traits::EncodeAs::encode_as(__val) },
                ),
                XcoderSigil::Deref => (
                    quote_spanned! { span => self.#name },
                    quote_spanned! { span => *__val },
                ),
            };
            // --------------------------------------------------
            // pick the per-field encode strategy, emit its body, and skip optional fields when
            // absent. the temporaries each arm declares are scoped by wrapping the body in a block
            // (or the `if let Some` guard), so adjacent fields never collide
            // --------------------------------------------------
            let is_opt = crate::expand::helpers::is_option(ty);
            let arg = if is_opt { &opt_arg } else { &nonopt_arg };
            let strategy = field_encode_strategy(attrs.fallback_enc, value_encoder, len_width);
            let body = emit_field_encode(
                &strategy,
                key_encoder,
                len_encoder,
                &enc_tokens,
                arg,
                key,
                span,
            );
            if is_opt {
                quote_spanned! { span => if let Some(ref __val) = self.#name { #body } }
            } else {
                quote_spanned! { span => { #body } }
            }
        });
    quote! { #(#items_encoded)* }
}

/// How a single field is written into the output buffer
///
/// Selected per field by [`field_encode_strategy`] from the value encoder and the container's
/// length encoder. All three produce byte-identical output; they trade off scratch allocation and
/// copies
enum FieldEncodeStrategy {
    /// Value byte width is a compile-time constant: write `key`, the constant length, then the
    /// value straight into `out` - no scratch, no length back-patch
    FixedWidth {
        /// The compile-time-known encoded byte width of the value
        value_width: usize,
    },
    /// Value width is unknown up front but the length encoder is fixed-width: reserve the length
    /// slot, write the value (or nested record) directly into `out`, then back-patch the length -
    /// no scratch buffer and no full-value copy
    Backpatch {
        /// The fixed byte width of the reserved length slot
        len_width: usize,
    },
    /// The length encoder is variable-width (e.g. BER): stage the value in `__scratch` to learn its
    /// byte length before the length prefix can be written
    Scratch,
}

/// Chooses the [`FieldEncodeStrategy`] for a field from its value encoder and the container's
/// length-encoder width
///
/// # Arguments
///
/// * `fallback_enc` - whether the field uses the nested `EncodeValue` fallback encoder
/// * `value_encoder` - the field's value encoder
/// * `len_width` - the container length encoder's fixed width, or `None` if variable-width
///
/// # Returns
///
/// `Scratch` when the length encoder is variable-width; otherwise `FixedWidth` for a non-fallback
/// field whose value encoder is a recognized fixed-width primitive writer, else `Backpatch`
fn field_encode_strategy(
    fallback_enc: bool,
    value_encoder: &types::SiguledXcoder,
    len_width: Option<usize>,
) -> FieldEncodeStrategy {
    match len_width {
        None => FieldEncodeStrategy::Scratch,
        Some(len_width) => {
            if !fallback_enc && let Some(value_width) = fixed_value_enc_width(value_encoder) {
                return FieldEncodeStrategy::FixedWidth { value_width };
            }
            FieldEncodeStrategy::Backpatch { len_width }
        }
    }
}

/// Emits the per-field encode statements for a [`FieldEncodeStrategy`]
///
/// The returned tokens reference `out` (and, for [`FieldEncodeStrategy::Scratch`], `__scratch`),
/// which the surrounding `encode_value` body provides. The caller wraps the result in a block or
/// an `if let Some(..)` guard, so the `__`-prefixed temporaries are scoped per field
///
/// # Arguments
///
/// * `strategy` - the chosen encode strategy
/// * `key_encoder` - the container key encoder
/// * `len_encoder` - the container length encoder
/// * `enc_tokens` - the value encoder call target (a leaf path or a nested `encode_value`)
/// * `arg` - the sigil-shaped value argument (`&self.field`, `__val`, etc.)
/// * `key` - the field's key literal
/// * `span` - the field name span, for diagnostics
///
/// # Returns
///
/// The token stream that writes this field's `key`, length, and value into `out`
fn emit_field_encode(
    strategy: &FieldEncodeStrategy,
    key_encoder: &types::SiguledXcoder,
    len_encoder: &types::SiguledXcoder,
    enc_tokens: &proc_macro2::TokenStream,
    arg: &proc_macro2::TokenStream,
    key: &syn::Lit,
    span: proc_macro2::Span,
) -> proc_macro2::TokenStream {
    match *strategy {
        // --------------------------------------------------
        // fixed-width value: the length is the compile-time constant `value_width`, written before
        // the value with no scratch and no back-patch. the `assert_eq!` (release-active, not a
        // debug-assert) enforces the width contract: a mis-detected encoder that wrote a different
        // count aborts loudly here instead of leaving a wrong length prefix in the stream. with
        // `panic = "abort"` no partially-written buffer is ever observed by a caller
        // --------------------------------------------------
        FieldEncodeStrategy::FixedWidth { value_width } => quote_spanned! { span =>
            #key_encoder(#key, out);
            #len_encoder(#value_width, out);
            let __vstart = out.len();
            #enc_tokens(#arg, out);
            assert_eq!(
                out.len() - __vstart,
                #value_width,
                "tinyklv: fixed-width value encoder wrote a different byte count than its detected width",
            );
        },
        // --------------------------------------------------
        // unknown value width, fixed-width length: reserve the length slot, write the value (or
        // nested record) directly into `out`, then back-patch the length. no scratch, no full-value
        // copy. shares the slot-reserve/back-patch mechanism with the frame encoder
        // --------------------------------------------------
        FieldEncodeStrategy::Backpatch { len_width } => emit_len_backpatch(
            quote_spanned! { span => #key_encoder(#key, out); },
            len_encoder,
            len_width,
            quote_spanned! { span => #enc_tokens(#arg, out); },
        ),
        // --------------------------------------------------
        // variable-width length (BER): stage the value in `__scratch` to learn its byte length,
        // then write key + length + the staged bytes
        // --------------------------------------------------
        FieldEncodeStrategy::Scratch => quote_spanned! { span =>
            __scratch.clear();
            #enc_tokens(#arg, &mut __scratch);
            #key_encoder(#key, out);
            #len_encoder(__scratch.len(), out);
            out.extend_from_slice(&__scratch);
        },
    }
}

/// Spare capacity reserved beyond the staged body for a variable-width (BER) length prefix
///
/// A BER long-form length is `0x80 | num_bytes` followed by up to 8 significant bytes, so a few
/// extra reserved bytes avoid a realloc when the prefix is written. A `Vec::reserve` hint only,
/// so the exact value is not load-bearing
const BER_LEN_PREFIX_ALLOWANCE: usize = 8;

/// Emits a length-back-patched KLV write into `out`
///
/// Writes `prefix` (the key or sentinel), reserves a fixed-width length slot, writes `body`
/// directly into `out`, then encodes the real length at the tail and moves it into the reserved
/// slot (a non-overlapping memmove), dropping the tail copy. Shared by the `EncodeFrame` body and
/// the per-field [`FieldEncodeStrategy::Backpatch`] arm so the slot arithmetic lives in one place.
/// The `debug_assert_eq!` enforces that the length encoder wrote the detected `len_width`
///
/// # Arguments
///
/// * `prefix` - tokens written before the length slot (the field key, or `reserve` + sentinel)
/// * `len_encoder` - the container length encoder (writes the length once the body length is known)
/// * `len_width` - the fixed byte width of the reserved length slot
/// * `body` - tokens that write the value (or nested record) directly into `out`
///
/// # Returns
///
/// The token stream performing the reserve / write-body / back-patch sequence
fn emit_len_backpatch(
    prefix: proc_macro2::TokenStream,
    len_encoder: &types::SiguledXcoder,
    len_width: usize,
    body: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    quote! {
        #prefix
        // reserve the fixed-width length slot, then write the body directly after it
        let __len_pos = out.len();
        out.resize(__len_pos + #len_width, 0u8);
        let __body_start = out.len();
        #body
        let __body_len = out.len() - __body_start;
        // a body longer than the fixed-width length prefix can represent would otherwise wrap
        // silently in the `*_from_usize` length encoder's `as` cast (e.g. `300 as u8 == 44`),
        // emitting a length prefix that lies about the body - a corrupt, mis-framed packet. fail
        // loudly instead. on the variable-width (non-hot) field path, so the check is negligible.
        // the `>= size_of::<usize>()` short-circuit avoids a `1 << 64` overflow for wide slots
        assert!(
            #len_width >= ::core::mem::size_of::<usize>() || __body_len < (1usize << (8 * #len_width)),
            "tinyklv: encoded body length ({} bytes) exceeds the {}-byte KLV length prefix",
            __body_len,
            #len_width,
        );
        // encode the real length at the tail with the length encoder, then move its bytes into the
        // reserved slot (non-overlapping: the slot ends exactly where the body begins) and truncate
        let __tail = out.len();
        #len_encoder(__body_len, out);
        debug_assert_eq!(
            out.len() - __tail,
            #len_width,
            "length encoder wrote a different byte count than its detected fixed width",
        );
        out.copy_within(__tail.., __len_pos);
        out.truncate(__tail);
    }
}

/// Returns `true` when the container has at least one `#[klv(..)]`-attributed field
///
/// Used to decide whether the generated `encode_value` body needs the per-field
/// value-staging scratch buffer; a field-less container omits it to avoid an
/// unused-variable lint
///
/// # Arguments
///
/// * `input` - The parsed container descriptor
///
/// # Returns
///
/// `true` if any field carries a `#[klv(..)]` attribute, `false` otherwise
fn has_attributed_fields(input: &MainContainer) -> bool {
    input.data.iter().any(|field| field.attrs.is_some())
}

/// Computes a compile-time capacity lower bound for the encoded value buffer
///
/// Sums the fixed byte width of every attributed field whose type is a known
/// fixed-width primitive, plus a small per-field allowance for the key and length
/// bytes. Variable-width fields (strings, `Vec`, nested records, BER) and optional
/// fields contribute only the per-field allowance. The result is passed to
/// `Vec::reserve`, so an under- or over-estimate only affects the reallocation
/// count, never correctness
///
/// # Arguments
///
/// * `input` - The parsed container descriptor
///
/// # Returns
///
/// The capacity hint, in bytes
fn gen_reserve_hint(input: &MainContainer) -> usize {
    /// Per-field key + length byte allowance folded into the reserve hint
    ///
    /// Rough accounting for the typical 1-byte key + 1-byte length prefix each field adds;
    /// the result is only a `Vec::reserve` hint, so the exact value is not critical
    const KEY_LEN_ALLOWANCE: usize = 2;
    input
        .data
        .iter()
        .filter(|field| field.attrs.is_some())
        .map(|field| KEY_LEN_ALLOWANCE + fixed_value_width(field.ty).unwrap_or(0))
        .sum()
}

/// Returns the encoded byte width of a fixed-width primitive type name, or `None`
///
/// Single source of truth for the primitive -> byte-width mapping shared by the value-field
/// reserve hint ([`fixed_value_width`]) and the length-encoder fixed-width detection
/// ([`fixed_len_width`])
///
/// # Arguments
///
/// * `name` - The primitive type name (e.g. `"u32"`)
///
/// # Returns
///
/// `Some(width)` for a recognized fixed-width primitive, `None` otherwise
fn primitive_width(name: &str) -> Option<usize> {
    Some(match name {
        "u8" | "i8" => 1,
        "u16" | "i16" => 2,
        "u32" | "i32" | "f32" => 4,
        "u64" | "i64" | "f64" => 8,
        "u128" | "i128" => 16,
        _ => return None,
    })
}

/// Returns the encoded byte width of a fixed-width primitive field type, or `None`
///
/// Recognizes the standard integer and float primitives. `Option<T>` and any
/// non-primitive (strings, `Vec`, nested records) return `None`, since their
/// encoded width is not known at compile time. The width is inferred from the
/// type alone, so a lengthed or ASCII encoder on a primitive may not match it -
/// acceptable, because the result is only a `Vec::reserve` hint
///
/// # Arguments
///
/// * `ty` - The field type to inspect
///
/// # Returns
///
/// `Some(width)` for a recognized fixed-width primitive, `None` otherwise
fn fixed_value_width(ty: &syn::Type) -> Option<usize> {
    if helpers::is_option(ty) {
        return None;
    }
    let syn::Type::Path(path) = ty else {
        return None;
    };
    primitive_width(&path.path.get_ident()?.to_string())
}

/// Strips a leading `be_`/`le_` endianness prefix from an encoder name
///
/// # Arguments
///
/// * `name` - the encoder's last path segment (e.g. `"be_u32"`, `"u8_from_usize"`)
///
/// # Returns
///
/// `(remainder, true)` when a `be_`/`le_` prefix was present, else `(name, false)`
fn strip_endian(name: &str) -> (&str, bool) {
    match name
        .strip_prefix("be_")
        .or_else(|| name.strip_prefix("le_"))
    {
        Some(rest) => (rest, true),
        None => (name, false),
    }
}

/// Returns the fixed encoded byte width of a length encoder, or `None` if variable-width
///
/// Recognizes the built-in `*_from_usize` integer/float length encoders, whose output width
/// is constant regardless of the value. Everything else - notably the BER length encoder -
/// is treated as variable-width. Used to decide whether `encode_frame` can reserve the length
/// slot up front and back-patch it (fixed-width), or must stage the body in a buffer to learn
/// its byte length first (variable-width)
///
/// Detection is by encoder name (the last path segment), so the returned width is a *contract*:
/// the named encoder must append exactly that many bytes. The built-in `*_from_usize` encoders
/// satisfy this; the generated `encode_frame` `debug_assert!`s it, so a mis-detected encoder
/// fails loudly in debug builds rather than emitting a wrong-width length prefix
///
/// # Arguments
///
/// * `len_encoder` - the container's length encoder
///
/// # Returns
///
/// `Some(width)` for a recognized fixed-width `*_from_usize` encoder, `None` otherwise
fn fixed_len_width(len_encoder: &types::SiguledXcoder) -> Option<usize> {
    let types::XcoderLike::Path(path) = &len_encoder.inner else {
        return None;
    };
    let ident = path.segments.last()?.ident.to_string();
    // the endianness prefix is optional for a length encoder (bare `u8_from_usize` is native-endian)
    let (stem, _prefixed) = strip_endian(ident.strip_suffix("_from_usize")?);
    primitive_width(stem)
}

/// Returns the encoded byte width of a *value* encoder when it is unambiguously fixed-width, else `None`
///
/// Recognizes only the endianness-prefixed binary primitive writers (`be_u32`, `le_i16`, `be_f64`, …),
/// whose output width is a compile-time constant. Bare names (`u8`, `u32`) are deliberately *not*
/// matched: they collide with the ASCII text encoders (`codecs::string::enc::u32`, which writes a
/// variable-length decimal), and `*_lengthed` is a variable-width closure factory — those, and every
/// custom/nested encoder, take the always-correct back-patch path instead.
///
/// Detection is by name, so the returned width is a *contract*: the named encoder must append
/// exactly that many bytes. The built-in `be_`/`le_` writers satisfy it; a user encoder named
/// `be_*`/`le_*` that lies about its width is caught by the release-active `assert_eq!` in the
/// generated `FixedWidth` arm (it aborts rather than emitting a wrong length prefix), not silently
///
/// # Arguments
///
/// * `value_encoder` - the field's value encoder
///
/// # Returns
///
/// `Some(width)` for a recognized `be_`/`le_`-prefixed fixed-width primitive encoder, `None` otherwise
fn fixed_value_enc_width(value_encoder: &types::SiguledXcoder) -> Option<usize> {
    let types::XcoderLike::Path(path) = &value_encoder.inner else {
        return None;
    };
    let ident = path.segments.last()?.ident.to_string();
    let (stem, prefixed) = strip_endian(&ident);
    if !prefixed {
        return None;
    }
    primitive_width(stem)
}
