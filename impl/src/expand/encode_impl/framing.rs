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
//! * [`EncodeFraming::Fused`] stages every value, then hands `(key, value_len)` to the `keylen`
//!   encoder, which writes the key and, when the key implies one, the length
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{fixed_width, length_prefix::LengthPrefix, staging::Staging};
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
use quote::{ToTokens, quote_spanned};

// --------------------------------------------------
// constants
// --------------------------------------------------
/// Fallback per-field key-byte reserve estimate
const DEFAULT_KEY_BYTES: usize = 1;

/// Fallback per-field length-byte reserve estimate
const DEFAULT_LEN_BYTES: usize = 1;

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
    /// The total encode reserve in bytes for all encoded value fields in the container
    pub(super) fn value_reserve_bytes(self, input: &MainContainer) -> usize {
        // --------------------------------------------------
        // estimate per-field key and length bytes
        // --------------------------------------------------
        let (key_bytes, len_bytes) = self.header_reserve_bytes();
        // --------------------------------------------------
        // sum known field widths into the encode reserve
        // --------------------------------------------------
        input
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
            .fold(0, usize::saturating_add)
    }

    /// Declares the scratch buffer, when a field value is staged before its header is written
    ///
    /// A variable-width length prefix or a fused header encodes each field value into a scratch
    /// buffer before its frame header is written. A fixed-width prefix, or a `size(implicit)`
    /// value, writes directly to the caller output, so a container with no staged value declares
    /// nothing and keeps generated methods free of unused locals. A retained unknown frame is
    /// staged only under a fused header, which always declares the buffer
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
            Self::Fused { .. } => true,
        };
        staged.then(|| Staging::Scratch.declare())
    }

    /// Writes one frame: under a separate header, its key through the key encoder, then what
    /// `separate` writes after it; under a fused header, its value staged and the `keylen` header
    /// written from `(key, staged_len)` ([`KeylenEncoder::write_staged`])
    ///
    /// # Arguments
    ///
    /// * `key` - The frame's key expression
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
        value_write: impl FnOnce(&proc_macro2::TokenStream) -> proc_macro2::TokenStream,
        span: proc_macro2::Span,
        separate: impl FnOnce(proc_macro2::TokenStream, LengthPrefix<'a>) -> proc_macro2::TokenStream,
    ) -> proc_macro2::TokenStream {
        match self {
            Self::Separate {
                key_encoder, len, ..
            } => separate(key_encoder.write(key, span), len),
            Self::Fused { keylen_encoder } => {
                keylen_encoder.write_staged(Staging::Scratch, key, value_write, span)
            }
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
    /// Writes one frame whose value is staged first: stage, write the header, then append
    ///
    /// The value is encoded into the `staging` buffer, the header is written from `key` and the
    /// staged length, and the staged bytes follow it. The encoder call, staged length included, is
    /// located at the `keylen` encoder with the derive's call-site hygiene, so an encoder with the
    /// wrong signature is reported at the encoder while the derive's locals still resolve
    ///
    /// # Arguments
    ///
    /// * `staging` - The buffer the value is staged in
    /// * `key` - The frame's key expression
    /// * `value_write` - Builds the statements encoding the value into the buffer it is given
    /// * `span` - Span of the emitted write
    ///
    /// # Returns
    ///
    /// The staged write from [`Staging::write`]
    pub(super) fn write_staged(
        self,
        staging: Staging,
        key: &impl ToTokens,
        value_write: impl FnOnce(&proc_macro2::TokenStream) -> proc_macro2::TokenStream,
        span: proc_macro2::Span,
    ) -> proc_macro2::TokenStream {
        let Self(keylen_encoder) = self;
        let at = helpers::call_site_at(keylen_encoder);
        let mut buffer = staging.buffer();
        buffer.set_span(at);
        let header = quote_spanned! { at => #keylen_encoder((#key, #buffer.len()), out); };
        staging.write(header, value_write, span)
    }
}
