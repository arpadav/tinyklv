//! Field KLV item emission
//!
//! Turns each `#[klv(..)]` field into a complete key-length-value write. The
//! module resolves the encoder call shape, picks the write the container's framing
//! selects for the field, applies optional-field and skip guards, and hands the header write to
//! the container framing. The frames a `#[klv(unknown)]` field retains are written after the known
//! fields, through the same framing
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{
    EncodeCtx, UnknownEncoder,
    field_length_plan::{FieldLengthPlan, ValueWidth},
    fixed_width,
    length_prefix::LengthPrefix,
    staging::Staging,
};
use crate::{
    ast::{
        attr::{FieldParsed, MainField, size::SizeSpec, skip_if::SkipIf},
        types::{self, XcoderSigil},
    },
    expand::helpers,
};

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned;

/// [`EncodeCtx`] implementation of the field item writes
impl EncodeCtx<'_> {
    /// Generates all field KLV item writes for an `EncodeValue` body
    ///
    /// Walks the parsed container fields and emits one planned KLV item write for
    /// each keyed field. The `#[klv(unknown)]` field's frames are written by
    /// [`Self::unknown_items`], and fields without KLV attributes are ignored so ordinary Rust
    /// fields remain outside the generated wire output
    ///
    /// # Returns
    ///
    /// A token stream containing all generated field item writes in source order
    pub(super) fn klv_items(&self) -> proc_macro2::TokenStream {
        // --------------------------------------------------
        // emit one planned item for each attributed field
        // --------------------------------------------------
        let items = self
            .input
            .klv_fields()
            .map(|(field, attrs)| self.klv_item(&PlannedItemWrite { field, attrs }));
        // --------------------------------------------------
        // concatenate generated item writes
        // --------------------------------------------------
        quote! { #(#items)* }
    }

    /// Generates one `#[klv(..)]` field's complete KLV item write
    ///
    /// Picks the field write: implicit, staged, known or reserved. A `size(implicit)` value is
    /// written straight after its key with no length ([`PlannedItemWrite::write_implicit`]). A
    /// fused `keylen` encoder, or a variable-width length prefix, stages the value in the scratch
    /// buffer. Under a fixed-width length prefix the [`FieldLengthPlan`] picks a known-width or a
    /// reserved-slot write, and the fixed-width length slot writes it. A skipped field's whole
    /// write sits under its skip guard ([`PlannedItemWrite::skip_guard`])
    ///
    /// # Arguments
    ///
    /// * `item` - The field to write
    ///
    /// # Returns
    ///
    /// A token stream containing the field's complete generated KLV item write
    fn klv_item(&self, item: &PlannedItemWrite<'_>) -> proc_macro2::TokenStream {
        let span = item.span();
        let key = &item.attrs.key;
        let value_write = |dst: &proc_macro2::TokenStream| item.value_write(dst);
        let implicit = item
            .attrs
            .size
            .as_ref()
            .filter(|size| size.implicit().is_some());
        // --------------------------------------------------
        // emit the item write the container framing selects
        // --------------------------------------------------
        let item_write = self
            .framing
            .write_frame(key, value_write, span, |key_write, len| {
                match (implicit, len) {
                    (Some(size), _) => item.write_implicit(size, key_write),
                    (None, LengthPrefix::Fixed(len)) => match item.length_plan() {
                        FieldLengthPlan::KnownValueWidth(value) => {
                            len.write_known(key_write, value, value_write, span)
                        }
                        FieldLengthPlan::ReservedLengthSlot => {
                            len.write_backfilled(key_write, value_write, span)
                        }
                    },
                    (None, LengthPrefix::Variable(len)) => {
                        len.write_staged(Staging::Scratch, key_write, value_write, span)
                    }
                }
            });
        // --------------------------------------------------
        // wrap optional fields with their `Some` guard, then skipped fields with their skip
        // guard
        // --------------------------------------------------
        item.skip_guard(item.access().guard(item_write))
    }

    /// Generates the writes of the retained unknown frames, after the known fields, in the order the
    /// field holds them
    ///
    /// One loop over the `#[klv(unknown)]` field writes each `(key, value)` frame. Under a separate
    /// header the key encoder writes the key and the `unknown(enc = ..)` encoder writes the rest of
    /// the frame, its length included when the frame has one, straight into `out`. Under a fused
    /// header the value is staged and the `keylen` encoder writes the header from `(key,
    /// staged_len)` ([`KeylenEncoder::write_staged`](super::framing::KeylenEncoder::write_staged)).
    /// The key is passed by value, as a field's literal key is, so the key type is `Copy`. The
    /// loop is located at the field and the encoder call at the encoder, so a wrong element type
    /// or encoder signature is reported where it is written
    ///
    /// # Returns
    ///
    /// The loop, or `None` when the struct keeps no unknown frames
    pub(super) fn unknown_items(&self) -> Option<proc_macro2::TokenStream> {
        let UnknownEncoder { encoder, keep } = self.unknown?;
        let name = &keep.name;
        let span = helpers::call_site_at(name);
        let at = helpers::call_site_at(&encoder.inner);
        let key = quote_spanned! { span => *__key };
        let value = ValueAccess::Frame {
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
        let frame_write = self
            .framing
            .write_frame(&key, value_write, span, |key_write, _| {
                let value_write = value_write(&quote_spanned! { span => out });
                quote! { #key_write #value_write }
            });
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
    /// A retained unknown frame, whose re-emit loop binds the value to a local
    Frame {
        /// Where the argument is located: the `unknown(enc = ..)` encoder
        at: proc_macro2::Span,
        /// The local the loop binds the value to
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
    /// optional one, of the loop's local for a retained unknown frame
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
                let span = name.span();
                return match sigil {
                    XcoderSigil::None => quote_spanned! { span => &self.#name },
                    XcoderSigil::Ref => quote_spanned! { span =>
                        ::tinyklv::traits::EncodeAs::encode_as(&self.#name)
                    },
                    XcoderSigil::Deref => quote_spanned! { span => self.#name },
                };
            }
            Self::Optional { name, binding } => (name.span(), binding),
            Self::Frame { at, binding } => (*at, binding),
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

    /// Wraps an optional field's write in its `Some` guard; a required field's or an unknown
    /// frame's is unchanged
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
            Self::Required { .. } | Self::Frame { .. } => item_write,
            Self::Optional { name, binding } => quote_spanned! { name.span() =>
                if let ::core::option::Option::Some(ref #binding) = self.#name {
                    #item_write
                }
            },
        }
    }
}

/// One `#[klv(..)]` field, from which every input of its item write is derived
struct PlannedItemWrite<'a> {
    /// The field, whose value type's primitive width may fix the value length
    field: &'a MainField<'a>,

    /// The field's `#[klv(..)]` attributes: its key, and the size and encoder that may fix the
    /// value length
    attrs: &'a FieldParsed,
}

/// [`PlannedItemWrite`] implementation
impl<'a> PlannedItemWrite<'a> {
    /// Source span used to attach diagnostics to generated field code
    ///
    /// # Returns
    ///
    /// The span of the field's name
    fn span(&self) -> proc_macro2::Span {
        self.field.name.span()
    }

    /// How the generated write reaches the field's value
    ///
    /// # Returns
    ///
    /// The field's [`ValueAccess`]
    fn access(&self) -> ValueAccess<'a> {
        ValueAccess::new(self.field)
    }

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

    /// The field's value encoder
    ///
    /// # Returns
    ///
    /// `attrs.enc`
    ///
    /// # Panics
    ///
    /// When the field has no value encoder. [`EncodeCtx::new`] returns before item generation
    /// unless every field has one
    fn value_encoder(&self) -> &'a types::SiguledXcoder {
        #[allow(
            clippy::unwrap_used,
            reason = "`EncodeCtx::new` returns before item generation unless every field has an encoder"
        )]
        self.attrs.enc.as_ref().unwrap()
    }

    /// Builds the encoder path used for the field value
    ///
    /// Chooses between the fallback `EncodeValue::encode_value` method for the
    /// field's value type and the explicit encoder expression declared in the field
    /// attributes
    ///
    /// # Returns
    ///
    /// A token stream containing the encoder callable used for the field value
    fn value_encoder_call(&self) -> proc_macro2::TokenStream {
        // --------------------------------------------------
        // resolve fallback trait encoder or explicit encoder path
        // --------------------------------------------------
        if self.attrs.fallback_enc {
            let ty = self.field.value_ty();
            quote! { <#ty as ::tinyklv::traits::EncodeValue>::encode_value }
        } else {
            let inner = &self.value_encoder().inner;
            quote! { #inner }
        }
    }

    /// Chooses how the field's value length is written under a fixed-width length prefix
    ///
    /// Prefers writing the length immediately when an exact field size is declared, or when a
    /// primitive type and its built-in encoder agree on a width. Otherwise the container's fixed
    /// length slot is reserved and back-filled after the value is encoded
    ///
    /// # Returns
    ///
    /// The known value width when provable, else the reserved fixed slot
    fn length_plan(&self) -> FieldLengthPlan<'a> {
        // --------------------------------------------------
        // explicit exact field sizes can be written immediately
        // --------------------------------------------------
        if let Some(size) = &self.attrs.size
            && let Some(bytes) = size.exact_width()
        {
            return FieldLengthPlan::KnownValueWidth(ValueWidth {
                field: &self.field.name,
                bytes,
                declared_at: size.span(),
            });
        }
        // --------------------------------------------------
        // prove fixed width from primitive type and built-in encoder
        // --------------------------------------------------
        if !self.attrs.fallback_enc {
            let encoder = &self.value_encoder().inner;
            if let (Some(type_width), Some(encoder_width)) = (
                fixed_width::fixed_value_type_width(self.field.value_ty()),
                fixed_width::builtin_fixed_encoder_width(encoder),
            ) && type_width == encoder_width
            {
                return FieldLengthPlan::KnownValueWidth(ValueWidth {
                    field: &self.field.name,
                    bytes: type_width,
                    declared_at: encoder.span(),
                });
            }
        }
        // --------------------------------------------------
        // otherwise reserve the container's fixed length slot
        // --------------------------------------------------
        FieldLengthPlan::ReservedLengthSlot
    }

    /// Writes a prefix, then the `size(implicit)` value straight into `out`, with no length
    ///
    /// Under `exact = N` the value write is checked in debug builds to be `N` bytes
    /// ([`ValueWidth::write_after`])
    ///
    /// # Arguments
    ///
    /// * `size` - The field's `size(..)`
    /// * `prefix_write` - Generated statements that write the field key
    ///
    /// # Returns
    ///
    /// A block writing the complete frame into `out`
    fn write_implicit(
        &self,
        size: &SizeSpec,
        prefix_write: proc_macro2::TokenStream,
    ) -> proc_macro2::TokenStream {
        let span = self.span();
        let value_write = |dst: &proc_macro2::TokenStream| self.value_write(dst);
        // --------------------------------------------------
        // a value of no declared width is written unchecked
        // --------------------------------------------------
        let Some(bytes) = size.exact_width() else {
            let value_write = value_write(&quote_spanned! { span => out });
            return quote_spanned! { span =>
                {
                    #prefix_write
                    #value_write
                }
            };
        };
        // --------------------------------------------------
        // an `exact = N` value is checked to be `N` bytes
        // --------------------------------------------------
        let value = ValueWidth {
            field: &self.field.name,
            bytes,
            declared_at: size.span(),
        };
        value.write_after(prefix_write, value_write, span)
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
        let value_argument = self.access().argument(self.value_encoder().sigil);
        quote_spanned! { self.span() => #value_encoder_call(#value_argument, #dst); }
    }
}
