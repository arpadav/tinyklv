//! Field KLV item emission
//!
//! Turns each `#[klv(..)]` field into a complete key-length-value write. The
//! module resolves the encoder call shape, writes each key's frame through the
//! [`FrameBody`](crate::expand::encode_impl::frame_body::FrameBody) the container's framing and
//! the key's value pick, and applies optional-field and skip guards. A
//! `key = oneof(..)` field writes the frame of the arm its variant
//! names. The frames a `#[klv(unknown)]` field retains are written after the known
//! fields, through the same framing, and a field with a `break_on` after them all
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{EncodeCtx, UnknownEncoder, frame_body::FrameBody};
use crate::{
    ast::{
        attr::{FieldKey, FieldKeys, FieldParsed, MainField, ValueXcoder, skip_if::SkipIf},
        types::{self, XcoderSigil},
    },
    expand::helpers,
};

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::{format_ident, quote, quote_spanned};

/// [`EncodeCtx`] implementation of the field item writes
impl EncodeCtx<'_> {
    /// Generates every frame write of an `EncodeValue` body
    ///
    /// Walks the parsed container fields and emits one planned KLV item write for
    /// each keyed field, in source order, then the frames the `#[klv(unknown)]` field retains
    /// ([`Self::unknown_items`]). A field with a `break_on` is written last, after them all: its
    /// `break_on` may end the decode, which then never reads a frame written after it. Fields
    /// without KLV attributes are ignored so ordinary Rust fields remain outside the generated
    /// wire output
    ///
    /// # Returns
    ///
    /// A token stream containing every generated frame write, or `None` when a key has no value
    /// encoder (`allow_unimplemented_encode`)
    pub(super) fn klv_items(&self) -> Option<proc_macro2::TokenStream> {
        // --------------------------------------------------
        // one planned item for each attributed field, with or without a `break_on`
        // --------------------------------------------------
        let items = |breaks: bool| {
            self.input
                .keyed_fields()
                .filter(move |(_, attrs)| attrs.break_on.is_some() == breaks)
                .map(|(field, attrs)| self.klv_item(&FieldWrite { field, attrs }))
                .collect::<Option<proc_macro2::TokenStream>>()
        };
        // --------------------------------------------------
        // the fields, the retained unknown frames, then the fields with a `break_on`
        // --------------------------------------------------
        let fields = items(false)?;
        let unknown = self.unknown_items();
        let breaking = items(true)?;
        Some(quote! {
            #fields
            #unknown
            #breaking
        })
    }

    /// Generates one `#[klv(..)]` field's complete KLV item write
    ///
    /// The write is its key's frame ([`Self::frame_write`]), or a `key = oneof(..)` field's
    /// `match` on its variant, whose arm per variant writes that arm's frame:
    ///
    /// ```text
    /// match #value {
    ///     #variant(__val) => { #frame_write }
    /// }
    /// ```
    ///
    /// `#value` is a reference to the field's value. The `match` is located at the field, so a
    /// variant no arm names is reported there. An optional field's write sits under its `Some`
    /// guard, and a skipped field's whole write under its skip guard
    /// ([`FieldWrite::skip_guard`])
    ///
    /// # Arguments
    ///
    /// * `item` - The field to write
    ///
    /// # Returns
    ///
    /// A token stream containing the field's complete generated KLV item write, or `None` when a
    /// key has no value encoder
    fn klv_item(&self, item: &FieldWrite<'_>) -> Option<proc_macro2::TokenStream> {
        let FieldWrite { field, attrs } = *item;
        let access = ValueAccess::new(field);
        let write = match &attrs.keys {
            FieldKeys::One { .. } => FieldKey::of(field, attrs)
                .map(|key| self.frame_write(key, &access))
                .collect::<Option<proc_macro2::TokenStream>>()?,
            FieldKeys::Oneof(oneof) => {
                // --------------------------------------------------
                // the reference the `match` reads, located at the field, where a variant no arm
                // names is reported
                // --------------------------------------------------
                let at = helpers::call_site_at(&field.name);
                let value: proc_macro2::TokenStream = access
                    .argument(XcoderSigil::None)
                    .into_iter()
                    .map(|mut token| {
                        token.set_span(at);
                        token
                    })
                    .collect();
                // --------------------------------------------------
                // each arm's variant and the frame of its key, which writes the value the
                // variant's pattern binds, located at the arm's type
                // --------------------------------------------------
                let arms = oneof
                    .iter()
                    .map(|oneof_key| {
                        let key = FieldKey::arm(field, attrs, oneof_key);
                        let bound_at = helpers::call_site_at(key.ty);
                        let bound = ValueAccess::Bound {
                            at: bound_at,
                            binding: format_ident!("__val", span = bound_at),
                        };
                        let frame_write = self.frame_write(key, &bound)?;
                        let variant = &oneof_key.arm.variant;
                        Some(quote_spanned! { at => #variant(__val) => { #frame_write } })
                    })
                    .collect::<Option<proc_macro2::TokenStream>>()?;
                quote_spanned! { at =>
                    match #value {
                        #arms
                    }
                }
            }
        };
        // --------------------------------------------------
        // wrap optional fields with their `Some` guard, then skipped fields with their skip
        // guard
        // --------------------------------------------------
        Some(item.skip_guard(access.guard(write)))
    }

    /// Generates one key's frame write, through the [`FrameBody`] the container's framing and the
    /// key's value pick ([`FrameBody::of`])
    ///
    /// # Arguments
    ///
    /// * `key` - The key to write
    /// * `access` - How the write reaches the key's value
    ///
    /// # Returns
    ///
    /// A token stream containing the key's generated frame write, or `None` when the key has no
    /// value encoder
    fn frame_write(
        &self,
        key: FieldKey<'_>,
        access: &ValueAccess<'_>,
    ) -> Option<proc_macro2::TokenStream> {
        let item = PlannedItemWrite {
            encoder: key.codec.enc.as_ref()?,
            key,
            access,
        };
        Some(FrameBody::of(self.framing, &item.key).write(
            item.key.key,
            |dst| item.value_write(dst),
            item.span(),
        ))
    }

    /// Generates the writes of the retained unknown frames, in the order the field holds them
    ///
    /// One loop over the `#[klv(unknown)]` field writes each `(key, value)` frame. Under a separate
    /// header the key encoder writes the key and the `unknown(enc = ..)` encoder writes the rest of
    /// the frame, its length included when the frame has one, straight into `out`. Under a fused
    /// header the value is staged and the `keylen` encoder writes the header from `(key,
    /// staged_len)` ([`FrameBody::unknown`]). The key is passed by value, as a field's literal key
    /// is, so the key type is `Copy`. The loop is located at the field and the encoder call at the
    /// encoder, so a wrong element type or encoder signature is reported where it is written
    ///
    /// # Returns
    ///
    /// The loop, or `None` when the struct keeps no unknown frames
    fn unknown_items(&self) -> Option<proc_macro2::TokenStream> {
        let UnknownEncoder { encoder, keep } = self.unknown?;
        let name = &keep.name;
        let span = helpers::call_site_at(name);
        let at = helpers::call_site_at(&encoder.inner);
        let key = quote_spanned! { span => *__key };
        let value = ValueAccess::Bound {
            at,
            binding: format_ident!("__val", span = at),
        }
        .argument(encoder.sigil);
        let inner = &encoder.inner;
        let value_write =
            |dst: &proc_macro2::TokenStream| quote_spanned! { at => #inner(#value, #dst); };
        // --------------------------------------------------
        // the frame write the container framing selects
        // --------------------------------------------------
        let frame_write = FrameBody::unknown(self.framing).write(&key, value_write, span);
        // --------------------------------------------------
        // one frame per retained `(key, value)`
        // --------------------------------------------------
        Some(quote_spanned! { span =>
            for (__key, __val) in &self.#name {
                #frame_write
            }
        })
    }
}

/// How the generated write reaches the value it encodes
enum ValueAccess<'a> {
    /// A required field, read through `self`
    Required {
        /// The field's identifier
        name: &'a syn::Ident,
    },
    /// An `Option<T>` field, whose `Some` guard binds the inner value to a local
    Optional {
        /// The field's identifier
        name: &'a syn::Ident,
        /// The local the guard binds the inner value to
        binding: syn::Ident,
    },
    /// A value bound to a local: a retained unknown frame's by its re-emit loop, a `oneof(..)`
    /// arm's by its variant's pattern
    Bound {
        /// Where the argument is located: the `unknown(enc = ..)` encoder, or the arm's type
        at: proc_macro2::Span,
        /// The local the value is bound to
        binding: syn::Ident,
    },
}

/// [`ValueAccess`] implementation
impl<'a> ValueAccess<'a> {
    /// Classifies a field by whether its type is `Option<T>`
    ///
    /// # Arguments
    ///
    /// * `field` - The field whose value is written
    ///
    /// # Returns
    ///
    /// [`Self::Optional`], binding `__val`, for an `Option<T>` field, else [`Self::Required`]
    fn new(field: &'a MainField) -> Self {
        let name = &field.name;
        if types::is_option(field.ty) {
            Self::Optional {
                name,
                binding: format_ident!("__val"),
            }
        } else {
            Self::Required { name }
        }
    }

    /// Shapes the field value argument according to the encoder sigil
    ///
    /// The sigil controls whether the encoder receives a reference, an `EncodeAs` projection, or a
    /// dereferenced value: of `self.field` for a required field, of the guard's local for an
    /// optional one, of the bound local for a retained unknown frame or a `oneof(..)` arm
    ///
    /// # Arguments
    ///
    /// * `sigil` - The value encoder's sigil
    ///
    /// # Returns
    ///
    /// The token expression passed to the value encoder
    fn argument(&self, sigil: XcoderSigil) -> proc_macro2::TokenStream {
        // --------------------------------------------------
        // build the argument form for the sigil
        // --------------------------------------------------
        let (span, binding) = match self {
            Self::Required { name } => {
                let span = helpers::call_site_at(name);
                return match sigil {
                    XcoderSigil::None => quote_spanned! { span => &self.#name },
                    XcoderSigil::Ref => quote_spanned! { span =>
                        ::tinyklv::traits::EncodeAs::encode_as(&self.#name)
                    },
                    XcoderSigil::Deref => quote_spanned! { span => self.#name },
                };
            }
            Self::Optional { name, binding } => (helpers::call_site_at(name), binding),
            Self::Bound { at, binding } => (*at, binding),
        };
        // --------------------------------------------------
        // a bound local is already a reference
        // --------------------------------------------------
        match sigil {
            XcoderSigil::None => quote_spanned! { span => #binding },
            XcoderSigil::Ref => quote_spanned! { span =>
                ::tinyklv::traits::EncodeAs::encode_as(#binding)
            },
            XcoderSigil::Deref => quote_spanned! { span => *#binding },
        }
    }

    /// Wraps an optional field's write in its `Some` guard; a required field's or a bound value's
    /// is unchanged
    ///
    /// Optional fields emit no KLV item when their value is `None`. The guard is spanned at the
    /// field, so diagnostics on it point back to the source field
    ///
    /// # Arguments
    ///
    /// * `item_write` - Generated item write to execute when the field is present
    ///
    /// # Returns
    ///
    /// A token stream containing either the guarded optional write or the original write
    fn guard(&self, item_write: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        // --------------------------------------------------
        // guard optional fields and write required fields directly
        // --------------------------------------------------
        match self {
            Self::Required { .. } | Self::Bound { .. } => item_write,
            Self::Optional { name, binding } => quote_spanned! { helpers::call_site_at(name) =>
                if let ::core::option::Option::Some(ref #binding) = self.#name {
                    #item_write
                }
            },
        }
    }
}

/// One `#[klv(..)]` field, whose frame writes sit under its guards
#[derive(Clone, Copy)]
struct FieldWrite<'a> {
    /// The field
    field: &'a MainField<'a>,

    /// The field's `#[klv(..)]` attributes: its keys and its skip
    attrs: &'a FieldParsed,
}

/// [`FieldWrite`] implementation
impl FieldWrite<'_> {
    /// Wraps the field's whole item write in its skip guard, when it has a [`SkipIf`]
    ///
    /// ```text
    /// // skip_if_default
    /// if self.#name != <#ty as Default>::default() { #item_write }
    /// // skip_if = #path
    /// if !#path(&self.#name) { #item_write }
    /// ```
    ///
    /// `#ty` is the field's declared type, so an `Option` field's `None` is its default. The
    /// guard is located at the field, so a type without `Default` or `PartialEq`, or a fn of
    /// another signature, is reported there
    ///
    /// # Arguments
    ///
    /// * `item_write` - The field's item write: key, length and value, under its `Some` guard
    ///
    /// # Returns
    ///
    /// The guarded write, or `item_write` unchanged without a skip
    fn skip_guard(&self, item_write: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        let Some(skip_if) = &self.attrs.skip_if else {
            return item_write;
        };
        let name = &self.field.name;
        let at = helpers::call_site_at(name);
        let written = match skip_if {
            SkipIf::Default(_) => {
                let ty = self.field.ty;
                quote_spanned! {at=> self.#name != <#ty as ::core::default::Default>::default() }
            }
            SkipIf::Path(path) => quote_spanned! {at=> !#path(&self.#name) },
        };
        quote_spanned! {at=>
            if #written {
                #item_write
            }
        }
    }
}

/// One key of a `#[klv(..)]` field, from which every input of its frame write is derived
struct PlannedItemWrite<'a> {
    /// The key, its field, and the type, size and encoder that may fix the value length
    key: FieldKey<'a>,

    /// The key's value encoder
    encoder: &'a ValueXcoder<types::SiguledXcoder>,

    /// How the write reaches the key's value: the field's, or a `oneof(..)` arm's bound local
    access: &'a ValueAccess<'a>,
}

/// [`PlannedItemWrite`] implementation
impl PlannedItemWrite<'_> {
    /// Source span used to attach diagnostics to generated field code
    ///
    /// # Returns
    ///
    /// The derive's call-site span located at the field's name
    /// ([`helpers::call_site_at`]), so the write's `self` and `out` resolve even when the name
    /// came in through a `macro_rules!` fragment
    fn span(&self) -> proc_macro2::Span {
        helpers::call_site_at(&self.key.field.name)
    }

    /// Builds the encoder path used for the key's value
    ///
    /// Chooses between the fallback `EncodeValue::encode_value` method for the
    /// value type and the explicit encoder expression declared in the field
    /// attributes
    ///
    /// # Returns
    ///
    /// A token stream containing the encoder callable used for the value
    fn value_encoder_call(&self) -> proc_macro2::TokenStream {
        // --------------------------------------------------
        // resolve fallback trait encoder or explicit encoder path
        // --------------------------------------------------
        match self.encoder {
            ValueXcoder::Explicit(encoder) => {
                let inner = &encoder.inner;
                quote! { #inner }
            }
            ValueXcoder::TraitFallback => {
                let ty = self.key.ty;
                quote! { <#ty as ::tinyklv::traits::EncodeValue>::encode_value }
            }
        }
    }

    /// The value encoder call writing the field value into `dst`
    ///
    /// # Arguments
    ///
    /// * `dst` - The `&mut Vec<u8>` expression the value is written into
    ///
    /// # Returns
    ///
    /// The value encoder call statement
    fn value_write(&self, dst: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        let value_encoder_call = self.value_encoder_call();
        // --------------------------------------------------
        // `EncodeValue::encode_value` takes the value by reference
        // --------------------------------------------------
        let sigil = match self.encoder {
            ValueXcoder::Explicit(encoder) => encoder.sigil,
            ValueXcoder::TraitFallback => XcoderSigil::None,
        };
        let value_argument = self.access.argument(sigil);
        quote_spanned! { self.span() => #value_encoder_call(#value_argument, #dst); }
    }
}
