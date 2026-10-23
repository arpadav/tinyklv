//! What each frame writes after its key, and the value widths known at compile time
//!
//! [`FrameBody`](crate::expand::encode_impl::frame_body::FrameBody) is the encode twin of decode's
//! `ValueSrc`: chosen once per item from the container's framing and, for a field's key, from its
//! value's size, type and encoder, it writes the item's frame.
//! [`ValueWidth`](crate::expand::encode_impl::frame_body::ValueWidth) is a value width known at
//! compile time, written as the value's length or checked in debug builds
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{
    fixed_width,
    framing::{EncodeFraming, KeyEncoder, KeylenEncoder},
    length_prefix::{FixedLengthSlot, KlvUnit, LengthPrefix, VariableLength},
    staging::Staging,
};
use crate::ast::{
    attr::{FieldKey, ValueXcoder},
    types::XcoderLike,
};

// --------------------------------------------------
// external
// --------------------------------------------------
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote, quote_spanned};
use syn::spanned::Spanned;

/// What a frame writes after its key, the encode twin of decode's `ValueSrc`
///
/// Each variant carries the encoders of the frame's header: the key encoder, with the length slot
/// or encoder when the value has a length, of a separate header, or the `keylen` encoder of a
/// fused one
#[derive(Debug, Clone, Copy)]
pub(super) enum FrameBody<'a> {
    /// A `size(implicit)` value, straight into `out` with no length, checked in debug builds to be
    /// its `exact = N` bytes
    Implicit(KeyEncoder<'a>, Option<ValueWidth<'a>>),
    /// The rest of a retained unknown frame, its length included when the frame has one, which the
    /// `unknown(enc = ..)` encoder writes straight into `out`
    Unknown(KeyEncoder<'a>),
    /// A value of known width under a fixed-width length slot: the width as its length, then the
    /// value, checked in debug builds to be that wide
    Known(KeyEncoder<'a>, FixedLengthSlot<'a>, ValueWidth<'a>),
    /// A value of expected width under a fixed-width length slot, and its built-in encoder's path:
    /// the width as its length, then the value, the slot back-filled when a user's encoder of that
    /// name writes another width
    Expected(
        KeyEncoder<'a>,
        FixedLengthSlot<'a>,
        ValueWidth<'a>,
        &'a syn::Path,
    ),
    /// A value of unknown width under a fixed-width length slot: the slot reserved, the value, then
    /// the slot back-filled
    Backfilled(KeyEncoder<'a>, FixedLengthSlot<'a>),
    /// A value under a variable-width length: staged in `__scratch`, then its length and the staged
    /// bytes, checked in debug builds to be its `exact = N` bytes
    Staged(KeyEncoder<'a>, VariableLength<'a>, Option<ValueWidth<'a>>),
    /// A value under a fused header: staged in `__scratch`, then the `keylen` header from `(key,
    /// staged_len)` and the staged bytes, checked in debug builds to be its `exact = N` bytes
    Keylen(KeylenEncoder<'a>, Option<ValueWidth<'a>>),
}

/// [`FrameBody`] implementation
impl<'a> FrameBody<'a> {
    /// The body of a field key's frame
    ///
    /// A fused header stages every value. Under a separate header a `size(implicit)` value has no
    /// length, a fixed-width length slot writes a value's known width ([`ValueWidth::exact`]) or
    /// expected width ([`ValueWidth::builtin`]) or is back-filled, and a variable-width length
    /// stages the value
    ///
    /// # Arguments
    ///
    /// * `framing` - The container's framing
    /// * `key` - The key whose frame is written
    ///
    /// # Returns
    ///
    /// The body of the key's frame
    pub(super) fn of(framing: EncodeFraming<'a>, key: &FieldKey<'a>) -> Self {
        let exact = ValueWidth::exact(key);
        let (key_encoder, len) = match framing {
            EncodeFraming::Separate {
                key_encoder, len, ..
            } => (key_encoder, len),
            EncodeFraming::Fused { keylen_encoder } => return Self::Keylen(keylen_encoder, exact),
        };
        // --------------------------------------------------
        // a `size(implicit)` value has no length
        // --------------------------------------------------
        if key.codec.implicit().is_some() {
            return Self::Implicit(key_encoder, exact);
        }
        match len {
            LengthPrefix::Fixed(slot) => match (exact, ValueWidth::builtin(key)) {
                (Some(width), _) => Self::Known(key_encoder, slot, width),
                (None, Some((width, encoder))) => Self::Expected(key_encoder, slot, width, encoder),
                (None, None) => Self::Backfilled(key_encoder, slot),
            },
            LengthPrefix::Variable(len) => Self::Staged(key_encoder, len, exact),
        }
    }

    /// The body of a retained unknown frame
    ///
    /// # Arguments
    ///
    /// * `framing` - The container's framing
    ///
    /// # Returns
    ///
    /// [`Self::Unknown`] under a separate header, else [`Self::Keylen`] with no width to check
    pub(super) fn unknown(framing: EncodeFraming<'a>) -> Self {
        match framing {
            EncodeFraming::Separate { key_encoder, .. } => Self::Unknown(key_encoder),
            EncodeFraming::Fused { keylen_encoder } => Self::Keylen(keylen_encoder, None),
        }
    }

    /// Whether the frame stages its value in [`Staging::Scratch`]
    ///
    /// # Returns
    ///
    /// `true` for [`Self::Staged`] and [`Self::Keylen`]
    pub(super) fn stages(self) -> bool {
        matches!(self, Self::Staged(..) | Self::Keylen(..))
    }

    /// The local the frame's value write calls its encoder through
    ///
    /// # Returns
    ///
    /// The [`fixed_width::encoder_binding`] of [`Self::Expected`], else `None`, the encoder called
    /// as written
    pub(super) fn encoder_binding(self) -> Option<syn::Ident> {
        let Self::Expected(.., encoder) = self else {
            return None;
        };
        Some(fixed_width::encoder_binding(encoder))
    }

    /// Writes the frame: its key, or the `keylen` header it goes into, and its value
    ///
    /// # Arguments
    ///
    /// * `key` - The frame's key expression
    /// * `value_write` - Builds the statements encoding the value into the buffer it is given
    /// * `span` - Span of the emitted write
    ///
    /// # Returns
    ///
    /// The frame write
    pub(super) fn write(
        self,
        key: &impl ToTokens,
        value_write: impl FnOnce(&TokenStream) -> TokenStream,
        span: Span,
    ) -> TokenStream {
        match self {
            Self::Implicit(key_encoder, None) => {
                let key_write = key_encoder.write(key, span);
                let value_write = value_write(&quote_spanned! { span => out });
                quote_spanned! { span =>
                    {
                        #key_write
                        #value_write
                    }
                }
            }
            Self::Implicit(key_encoder, Some(width)) => {
                width.write_after(key_encoder.write(key, span), value_write, span)
            }
            Self::Unknown(key_encoder) => {
                let key_write = key_encoder.write(key, span);
                let value_write = value_write(&quote_spanned! { span => out });
                quote! { #key_write #value_write }
            }
            Self::Known(key_encoder, slot, width) => {
                slot.write_known(key_encoder.write(key, span), width, value_write, span)
            }
            Self::Expected(key_encoder, slot, width, encoder) => {
                let key_write = key_encoder.write(key, span);
                let binding = fixed_width::encoder_binding(encoder);
                slot.write_expected(
                    quote! { #key_write let #binding = #encoder; },
                    width,
                    &fixed_width::foreign_encoder(encoder),
                    value_write,
                    span,
                )
            }
            Self::Backfilled(key_encoder, slot) => {
                let key_write = key_encoder.write(key, span);
                slot.write_backfilled(KlvUnit::Item, key_write, value_write, span)
            }
            Self::Staged(key_encoder, len, width) => {
                let prefix_write = key_encoder.write(key, span);
                let staged_write = Self::checked(value_write, width, span);
                len.write_staged(Staging::Scratch, prefix_write, staged_write, span)
            }
            Self::Keylen(keylen_encoder, width) => {
                let staged_write = Self::checked(value_write, width, span);
                keylen_encoder.write_staged(Staging::Scratch, key, staged_write, span)
            }
        }
    }

    /// The write staging a value in [`Staging::Scratch`], then, with a `width`, the debug-build
    /// check that the buffer holds its bytes ([`ValueWidth::check`])
    ///
    /// # Arguments
    ///
    /// * `value_write` - Builds the statements encoding the value into the buffer it is given
    /// * `width` - The value's `exact = N` width, if any
    /// * `span` - Span of the emitted check
    ///
    /// # Returns
    ///
    /// The value write, then the check
    fn checked(
        value_write: impl FnOnce(&TokenStream) -> TokenStream,
        width: Option<ValueWidth<'a>>,
        span: Span,
    ) -> impl FnOnce(&TokenStream) -> TokenStream {
        move |dst| {
            let value_write = value_write(dst);
            let check = width.map(|width| {
                let buffer = Staging::Scratch.buffer();
                width.check(&quote_spanned! { span => #buffer.len() }, span)
            });
            quote! {
                #value_write
                #check
            }
        }
    }
}

/// A field value width known at compile time
#[derive(Debug, Clone, Copy)]
pub(super) struct ValueWidth<'a> {
    /// The field whose value has this width
    pub(super) field: &'a syn::Ident,

    /// Exact encoded byte width of the field value
    pub(super) bytes: usize,

    /// The span that declared the width: the field's `size(..)`, else its value encoder
    pub(super) declared_at: proc_macro2::Span,
}

/// [`ValueWidth`] implementation
impl<'a> ValueWidth<'a> {
    /// A key's value width, as its `size(exact = N)` declares it
    ///
    /// # Arguments
    ///
    /// * `key` - The key whose value is written
    ///
    /// # Returns
    ///
    /// The width, `N` declared at its `size(..)`, or `None` without `exact = N`
    fn exact(key: &FieldKey<'a>) -> Option<Self> {
        let size = key.codec.size.as_ref()?;
        Some(Self {
            field: &key.field.name,
            bytes: size.exact_width()?,
            declared_at: size.span(),
        })
    }

    /// A key's value width, as a primitive type and its built-in fixed-width encoder agree on it
    ///
    /// # Arguments
    ///
    /// * `key` - The key whose value is written
    ///
    /// # Returns
    ///
    /// The width, declared at the encoder, and the encoder's path, or `None` under the trait
    /// fallback encoder, or when the type or the encoder has no fixed width, or the two widths
    /// differ
    fn builtin(key: &FieldKey<'a>) -> Option<(Self, &'a syn::Path)> {
        let Some(ValueXcoder::Explicit(encoder)) = &key.codec.enc else {
            return None;
        };
        let XcoderLike::Path(path) = &encoder.inner else {
            return None;
        };
        let bytes = fixed_width::fixed_value_type_width(key.ty)?;
        (fixed_width::builtin_fixed_encoder_width(&encoder.inner)? == bytes).then(|| {
            let width = Self {
                field: &key.field.name,
                bytes,
                declared_at: encoder.inner.span(),
            };
            (width, path)
        })
    }

    /// Writes a prefix, then the value, checking in debug builds that it is [`Self::bytes`] long
    ///
    /// # Arguments
    ///
    /// * `prefix_write` - Generated statements that write what precedes the value
    /// * `value_write` - Builds the statements appending the value bytes to the buffer it is given
    /// * `span` - Span of the emitted write
    ///
    /// # Returns
    ///
    /// A block writing the prefix and the value into `out`
    pub(super) fn write_after(
        self,
        prefix_write: TokenStream,
        value_write: impl FnOnce(&TokenStream) -> TokenStream,
        span: Span,
    ) -> TokenStream {
        let value_write = value_write(&quote_spanned! { span => out });
        let check = self.check(&quote_spanned! { span => out.len() - __value_start }, span);
        quote_spanned! { span =>
            {
                #prefix_write
                let __value_start = out.len();
                #value_write
                #check
            }
        }
    }

    /// Checks in debug builds that the value encoder wrote [`Self::bytes`] bytes
    ///
    /// # Arguments
    ///
    /// * `written` - The byte count the value encoder wrote
    /// * `span` - Span of the emitted check
    ///
    /// # Returns
    ///
    /// The `debug_assert_eq!`, its message naming [`Self::field`]
    fn check(self, written: &TokenStream, span: Span) -> TokenStream {
        let Self { field, bytes, .. } = self;
        let message = format!(
            "tinyklv: the value encoder of `{field}` wrote a different byte count than its fixed width"
        );
        quote_spanned! { span =>
            ::core::debug_assert_eq!(#written, #bytes, #message);
        }
    }
}
