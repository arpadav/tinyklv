//! The frame header written around each field value, per the container's framing mode
//!
//! [`EncodeFraming`] is the encode side of the container's framing mode:
//!
//! * [`EncodeFraming::Separate`] writes each field's key with the key encoder and its length with
//!   the length encoder. A variable-width length prefix stages every value; a fixed-width one
//!   ([`FixedLengthSlot`](crate::expand::encode_impl::length_prefix::FixedLengthSlot)) writes each
//!   value at a known width or back-fills its length. A `size(implicit)` value has no length, so
//!   it is written straight after its key; so is a retained unknown frame, whose encoder writes
//!   the frame's own length
//! * [`EncodeFraming::Fused`] hands `(key, value_len)` to the `keylen` encoder, which writes the
//!   key and, when the key implies one, the length: before a value of known width, else after the
//!   value, then rotated in front of it
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{
    field_length_plan::{FieldLengthPlan, ValueWidth},
    fixed_width,
    length_prefix::LengthPrefix,
    staging::Staging,
};
use crate::{
    ast::{
        attr::{
            MainContainer,
            container::ContainerMode,
            size::{SizeBytes, SizeSpec},
        },
        types,
    },
    expand::helpers,
};

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::{ToTokens, quote, quote_spanned};

// --------------------------------------------------
// constants
// --------------------------------------------------
/// Fallback per-field key-byte reserve estimate
const DEFAULT_KEY_BYTES: usize = 1;

/// Fallback per-field length-byte reserve estimate
const DEFAULT_LEN_BYTES: usize = 1;

/// The largest encode reserve estimate a `Vec::reserve` is emitted for: `i32::MAX`, the most bytes
/// a `Vec<u8>` holds on a 32-bit target, the narrowest that has `std`. `Vec::reserve` panics past
/// it there, so a larger estimate, which only a field's or length's `size(..)` of gigabytes makes,
/// is not reserved and the buffer grows as it is written
pub(super) const MAX_RESERVE_BYTES: usize = 0x7FFF_FFFF;

/// The container's frame header encoder(s), carrying the borrowed encoder token(s)
///
/// Mirrors [`ContainerMode`] on the encode codegen side
#[derive(Debug, Clone, Copy)]
pub(super) enum EncodeFraming<'a> {
    /// Separate key and length encoders
    Separate {
        /// Encoder writing each frame's key
        key_encoder: KeyEncoder<'a>,
        /// The length prefix written after each prefixed field's key and, under a sentinel, the
        /// frame length; a kept unknown frame's `unknown(..)` encoder writes its own
        len: LengthPrefix<'a>,
        /// The estimated bytes of one field's key, for the encode reserve
        key_bytes: usize,
        /// The estimated bytes of one field's length, for the encode reserve
        len_bytes: usize,
        /// The sentinel the `EncodeFrame` impl writes ahead of the frame length, if any
        sentinel: Option<&'a syn::Lit>,
    },
    /// One encoder writing each frame header from `(key, value_len)`
    Fused {
        /// Encoder expression writing each frame header
        keylen_encoder: KeylenEncoder<'a>,
    },
}

/// [`EncodeFraming`] implementation
impl<'a> EncodeFraming<'a> {
    /// Borrows the encoder token(s) of a container's framing mode and resolves its size shapes
    ///
    /// A separate header takes its key width from `key(.., size(..))` or a built-in fixed-width
    /// key encoder, and its length width from `len(.., size(..))`, each falling back to a one-byte
    /// default
    ///
    /// # Arguments
    ///
    /// * `mode` - The container's validated framing mode
    ///
    /// # Returns
    ///
    /// The framing, or `None` when an encoder is missing (`allow_unimplemented_encode`), in which
    /// case no encode impl is generated
    pub(super) fn from_mode(mode: &'a ContainerMode) -> Option<Self> {
        match mode {
            // --------------------------------------------------
            // separate key and length: each has its own encoder and width
            // --------------------------------------------------
            ContainerMode::Separate {
                key, len, sentinel, ..
            } => {
                let key_encoder = key.enc.as_ref()?;
                // --------------------------------------------------
                // the key width: its `size(..)`, else a built-in fixed-width encoder, else the
                // default
                // --------------------------------------------------
                let key_bytes = key
                    .size
                    .as_ref()
                    .and_then(SizeBytes::capacity_bytes)
                    .or_else(|| fixed_width::builtin_fixed_encoder_width(key_encoder))
                    .unwrap_or(DEFAULT_KEY_BYTES);
                // --------------------------------------------------
                // the length width: its `size(..)`, else the default
                // --------------------------------------------------
                let len_bytes = len
                    .size
                    .as_ref()
                    .and_then(SizeBytes::capacity_bytes)
                    .unwrap_or(DEFAULT_LEN_BYTES);
                // --------------------------------------------------
                // assemble the framing; a missing length encoder yields `None`
                // --------------------------------------------------
                Some(Self::Separate {
                    key_encoder: KeyEncoder(key_encoder),
                    len: LengthPrefix::new(len.enc.as_ref()?, len.size.as_ref()),
                    key_bytes,
                    len_bytes,
                    sentinel: sentinel.as_ref(),
                })
            }
            // --------------------------------------------------
            // fused: one `keylen` encoder writes each frame header
            // --------------------------------------------------
            ContainerMode::Fused { keylen, .. } => Some(Self::Fused {
                keylen_encoder: KeylenEncoder(keylen.enc.as_ref()?),
            }),
        }
    }

    /// Estimates the bytes one field's frame header occupies, for the encode reserve
    ///
    /// A fused header's widths are decided by the `keylen` encoder at run time, so both take the
    /// one-byte default
    ///
    /// # Returns
    ///
    /// The per-field key byte estimate, and the length byte estimate a value with a length prefix
    /// adds to it
    fn header_reserve_bytes(self) -> (usize, usize) {
        match self {
            Self::Separate {
                key_bytes,
                len_bytes,
                ..
            } => (key_bytes, len_bytes),
            Self::Fused { .. } => (DEFAULT_KEY_BYTES, DEFAULT_LEN_BYTES),
        }
    }

    /// Computes the `encode_value` encode reserve in bytes
    ///
    /// Estimates the number of bytes the generated `encode_value` method should reserve before
    /// writing field items: per key of a `#[klv(..)]` field, each `oneof(..)` arm's included, the
    /// frame header estimate ([`Self::header_reserve_bytes`]) plus any statically known value
    /// width. A `size(implicit)` value adds no length, and its width is its `exact = N` or
    /// `reserve = N`. Unknown widths contribute zero for the value portion, so the result is an
    /// estimate rather than a wire-format contract. Retained unknown frames add nothing: their
    /// count and widths are known only at run time. Every sum saturates at `usize::MAX`
    ///
    /// # Arguments
    ///
    /// * `input` - Parsed container metadata whose attributed fields are counted
    ///
    /// # Returns
    ///
    /// The total encode reserve in bytes for all encoded value fields in the container, or `None`
    /// when it is more than [`MAX_RESERVE_BYTES`]
    pub(super) fn value_reserve_bytes(self, input: &MainContainer) -> Option<usize> {
        // --------------------------------------------------
        // estimate per-field key and length bytes
        // --------------------------------------------------
        let (key_bytes, len_bytes) = self.header_reserve_bytes();
        // --------------------------------------------------
        // sum known field widths into the encode reserve, kept only within the largest
        // --------------------------------------------------
        let bytes = input
            .klv_keys()
            .map(|key| {
                let size = key.codec.size.as_ref();
                let (len, value) = match size.filter(|size| size.implicit().is_some()) {
                    Some(size) => (0, size.capacity_bytes()),
                    None => (
                        len_bytes,
                        fixed_width::fixed_value_type_width(key.field.ty)
                            .or_else(|| size.and_then(SizeSpec::capacity_bytes)),
                    ),
                };
                key_bytes
                    .saturating_add(len)
                    .saturating_add(value.unwrap_or(0))
            })
            .fold(0, usize::saturating_add);
        (bytes <= MAX_RESERVE_BYTES).then_some(bytes)
    }

    /// Declares the scratch buffer, when a field value is staged before its header is written
    ///
    /// A variable-width length prefix encodes each field value into a scratch buffer before its
    /// frame header is written. A fixed-width prefix, a `size(implicit)` value, or a fused header
    /// writes directly to the caller output, so a container with no staged value declares nothing
    /// and keeps generated methods free of unused locals. A retained unknown frame is never staged
    ///
    /// # Arguments
    ///
    /// * `input` - Parsed container metadata whose attributed fields are written
    ///
    /// # Returns
    ///
    /// [`Staging::Scratch`]'s empty `Vec<u8>` declaration when a value is staged, else `None`
    pub(super) fn declare_scratch(self, input: &MainContainer) -> Option<proc_macro2::TokenStream> {
        let staged = match self {
            Self::Separate { len, .. } => match len {
                LengthPrefix::Fixed(_) => false,
                LengthPrefix::Variable(_) => {
                    input.klv_keys().any(|key| key.codec.implicit().is_none())
                }
            },
            Self::Fused { .. } => false,
        };
        staged.then(|| Staging::Scratch.declare())
    }

    /// Writes one frame: under a separate header, its key through the key encoder, then what
    /// `separate` writes after it; under a fused header, a value of known width after the
    /// `keylen` header written from `(key, W)` ([`KeylenEncoder::write_known`]), any other value
    /// before the header written from `(key, value_len)` and rotated in front of it
    /// ([`KeylenEncoder::write_rotated`])
    ///
    /// # Arguments
    ///
    /// * `key` - The frame's key expression
    /// * `plan` - Whether the value's width is known before it is written
    /// * `value_write` - Builds the statements encoding the value into the buffer it is given
    /// * `span` - Span of the emitted write
    /// * `separate` - Builds the rest of a separate frame from its key write and the length
    ///   prefix
    ///
    /// # Returns
    ///
    /// The frame write
    pub(super) fn write_frame(
        self,
        key: &impl ToTokens,
        plan: FieldLengthPlan<'_>,
        value_write: impl FnOnce(&proc_macro2::TokenStream) -> proc_macro2::TokenStream,
        span: proc_macro2::Span,
        separate: impl FnOnce(proc_macro2::TokenStream, LengthPrefix<'a>) -> proc_macro2::TokenStream,
    ) -> proc_macro2::TokenStream {
        match self {
            Self::Separate {
                key_encoder, len, ..
            } => separate(key_encoder.write(key, span), len),
            Self::Fused { keylen_encoder } => match plan {
                FieldLengthPlan::KnownValueWidth(value) => {
                    keylen_encoder.write_known(key, value, value_write, span)
                }
                FieldLengthPlan::UnknownValueWidth => {
                    keylen_encoder.write_rotated(key, value_write, span)
                }
            },
        }
    }
}

/// The key encoder of a separate frame header
#[derive(Debug, Clone, Copy)]
pub(super) struct KeyEncoder<'a>(&'a types::XcoderType);

/// [`KeyEncoder`] implementation
impl KeyEncoder<'_> {
    /// Writes a frame's key into `out`
    ///
    /// # Arguments
    ///
    /// * `key` - The frame's key expression
    /// * `span` - Span of the emitted write
    ///
    /// # Returns
    ///
    /// The key encoder call statement
    pub(super) fn write(
        self,
        key: &impl ToTokens,
        span: proc_macro2::Span,
    ) -> proc_macro2::TokenStream {
        let Self(key_encoder) = self;
        quote_spanned! { span => #key_encoder(#key, out); }
    }
}

/// The `keylen` encoder of a fused frame header
#[derive(Debug, Clone, Copy)]
pub(super) struct KeylenEncoder<'a>(&'a types::XcoderType);

/// [`KeylenEncoder`] implementation
impl KeylenEncoder<'_> {
    /// Writes one frame whose value width is known: the header from `(key, W)`, then the value
    ///
    /// The value write is checked in debug builds to be `W` bytes ([`ValueWidth::write_after`])
    ///
    /// # Arguments
    ///
    /// * `key` - The frame's key expression
    /// * `value` - The value's known width
    /// * `value_write` - Builds the statements appending the value bytes to the buffer it is given
    /// * `span` - Span of the emitted write
    ///
    /// # Returns
    ///
    /// A block writing the complete frame into `out`
    pub(super) fn write_known(
        self,
        key: &impl ToTokens,
        value: ValueWidth<'_>,
        value_write: impl FnOnce(&proc_macro2::TokenStream) -> proc_macro2::TokenStream,
        span: proc_macro2::Span,
    ) -> proc_macro2::TokenStream {
        let bytes = value.bytes;
        let call = self.header(key);
        let header = quote! { let __value_len = #bytes; #call };
        value.write_after(header, value_write, span)
    }

    /// Writes one frame whose value width is unknown: the value, then the header from
    /// `(key, value_len)`, rotated in front of the value
    ///
    /// The header is written at the end of `out`, after the value, so no buffer stages the value.
    /// A one- or two-byte header is moved by shifting the value up with `copy_within` and storing
    /// its bytes in front of it; a longer one by `rotate_right`, which is not inlined and copies
    /// three times, a cost that only a short frame notices
    ///
    /// # Arguments
    ///
    /// * `key` - The frame's key expression
    /// * `value_write` - Builds the statements appending the value bytes to the buffer it is given
    /// * `span` - Span of the emitted write
    ///
    /// # Returns
    ///
    /// A block writing the complete frame into `out`
    pub(super) fn write_rotated(
        self,
        key: &impl ToTokens,
        value_write: impl FnOnce(&proc_macro2::TokenStream) -> proc_macro2::TokenStream,
        span: proc_macro2::Span,
    ) -> proc_macro2::TokenStream {
        let value_write = value_write(&quote_spanned! { span => out });
        let header = self.header(key);
        quote_spanned! { span =>
            {
                let __frame_start = out.len();
                #value_write
                let __header_start = out.len();
                let __value_len = __header_start - __frame_start;
                #header
                match out[__header_start..] {
                    [__first] => {
                        out.copy_within(__frame_start..__header_start, __frame_start + 1);
                        out[__frame_start] = __first;
                    }
                    [__first, __second] => {
                        out.copy_within(__frame_start..__header_start, __frame_start + 2);
                        out[__frame_start] = __first;
                        out[__frame_start + 1] = __second;
                    }
                    _ => {
                        let __header_len = out.len() - __header_start;
                        out[__frame_start..].rotate_right(__header_len);
                    }
                }
            }
        }
    }

    /// The header write from `(key, __value_len)`
    ///
    /// The encoder call is located at the `keylen` encoder with the derive's call-site hygiene,
    /// so an encoder with the wrong signature is reported at the encoder while the derive's
    /// `__value_len` and `out` still resolve
    ///
    /// # Arguments
    ///
    /// * `key` - The frame's key expression
    ///
    /// # Returns
    ///
    /// The encoder call statement
    fn header(self, key: &impl ToTokens) -> proc_macro2::TokenStream {
        let Self(keylen_encoder) = self;
        let at = helpers::call_site_at(keylen_encoder);
        quote_spanned! { at => #keylen_encoder((#key, __value_len), out); }
    }
}
