//! The frame header written around each field value, per the container's framing mode
//!
//! [`EncodeFraming`] is the encode side of the container's framing mode:
//!
//! * [`EncodeFraming::Separate`] writes each field's key with the key encoder and its length with
//!   the length encoder. A variable-width length prefix stages every value; a fixed-width one
//!   ([`FixedPrefix`]) writes each value at a known width or back-fills its length
//! * [`EncodeFraming::Fused`] stages every value, then hands `(key, value_len)` to the `keylen`
//!   encoder, which writes the key and, when the key implies one, the length
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{
    field_length_plan::{ContainerLengthPrefix, ValueWidth},
    fixed_width,
    length_slot_gen::FixedLengthSlot,
};
use crate::{
    ast::{
        attr::{MainContainer, container::ContainerMode, size::SizeSpec},
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
        /// Encoder writing each field's key
        key_encoder: KeyEncoder<'a>,
        /// Encoder expression writing each field's length and, under a sentinel, the frame length
        len_encoder: &'a types::XcoderType,
        /// The length prefix's shape, resolved from `len(.., size(..))`
        len_prefix: ContainerLengthPrefix,
        /// The estimated bytes of one field's key and length, for the reserve hint
        header_bytes: usize,
    },
    /// One encoder writing each frame header from `(key, value_len)`
    Fused {
        /// Encoder expression writing each frame header
        keylen_encoder: &'a types::XcoderType,
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
            ContainerMode::Separate { key, len } => {
                let key_encoder = key.enc.as_ref()?;
                let key_bytes = key
                    .size
                    .and_then(SizeSpec::capacity_bytes)
                    .or_else(|| fixed_width::builtin_fixed_encoder_width(key_encoder))
                    .unwrap_or(DEFAULT_KEY_BYTES);
                let len_bytes = len
                    .size
                    .and_then(SizeSpec::capacity_bytes)
                    .unwrap_or(DEFAULT_LEN_BYTES);
                Some(Self::Separate {
                    key_encoder: KeyEncoder(key_encoder),
                    len_encoder: len.enc.as_ref()?,
                    len_prefix: ContainerLengthPrefix::from_size(len.size),
                    header_bytes: key_bytes + len_bytes,
                })
            }
            ContainerMode::Fused(keylen) => Some(Self::Fused {
                keylen_encoder: keylen.enc.as_ref()?,
            }),
        }
    }

    /// Estimates the bytes one field's frame header occupies, for the reserve hint
    ///
    /// A fused header's widths are decided by the `keylen` encoder at run time, so both take the
    /// one-byte default
    ///
    /// # Returns
    ///
    /// The per-field key plus length byte estimate
    fn header_reserve_bytes(self) -> usize {
        match self {
            Self::Separate { header_bytes, .. } => header_bytes,
            Self::Fused { .. } => DEFAULT_KEY_BYTES + DEFAULT_LEN_BYTES,
        }
    }

    /// Computes the encode-value capacity reserve hint in bytes
    ///
    /// Estimates the number of bytes the generated `encode_value` method should reserve before
    /// writing field items: per `#[klv(..)]` field, the frame header estimate
    /// ([`Self::header_reserve_bytes`]) plus any statically known value width. Unknown widths
    /// contribute zero for the value portion, so the result is an allocation hint rather than a
    /// wire-format contract
    ///
    /// # Arguments
    ///
    /// * `input` - Parsed container metadata whose attributed fields are counted
    ///
    /// # Returns
    ///
    /// The total byte reserve hint for all encoded value fields in the container
    pub(super) fn value_reserve_bytes(self, input: &MainContainer) -> usize {
        // --------------------------------------------------
        // estimate per-field key and length bytes
        // --------------------------------------------------
        let header_bytes = self.header_reserve_bytes();
        // --------------------------------------------------
        // sum known field widths into the reserve hint
        // --------------------------------------------------
        input
            .data
            .iter()
            .filter_map(|field| field.attrs.as_ref().map(|attrs| (field.ty, attrs)))
            .map(|(ty, attrs)| {
                let value_bytes = fixed_width::fixed_value_type_width(ty)
                    .or_else(|| attrs.size.and_then(SizeSpec::capacity_bytes))
                    .unwrap_or(0);
                header_bytes + value_bytes
            })
            .sum()
    }

    /// The key encoder and fixed-width length slot of a fixed-width length prefix
    ///
    /// The one classifier of how field values are written: a fixed-width prefix writes each
    /// header directly, every other framing stages the value first
    ///
    /// # Returns
    ///
    /// The [`FixedPrefix`] of a separate header with a fixed-width length prefix, or `None` for a
    /// variable-width prefix and for every fused header, whose `keylen` encoder takes the value
    /// length
    pub(super) fn fixed_prefix(self) -> Option<FixedPrefix<'a>> {
        match self {
            Self::Separate {
                key_encoder,
                len_encoder,
                len_prefix: ContainerLengthPrefix::Fixed { width },
                ..
            } => Some(FixedPrefix {
                key_encoder,
                len: FixedLengthSlot { len_encoder, width },
            }),
            Self::Separate {
                len_prefix: ContainerLengthPrefix::Variable,
                ..
            }
            | Self::Fused { .. } => None,
        }
    }

    /// Whether field values are staged in a scratch buffer before their header is written
    ///
    /// # Returns
    ///
    /// `true` when the framing has no [`Self::fixed_prefix`]
    pub(super) fn stages_values(self) -> bool {
        self.fixed_prefix().is_none()
    }

    /// Writes one frame whose value is staged first: stage, write the header, then append
    ///
    /// The value is encoded into the cleared `scratch`, the header is written from `key` and the
    /// staged length, and the staged bytes follow it. A separate header is spanned at the field.
    /// The fused encoder call, staged length included, is located at the `keylen` encoder with
    /// the derive's call-site hygiene, so an encoder with the wrong signature is reported at the
    /// encoder while the derive's locals still resolve
    ///
    /// # Arguments
    ///
    /// * `key` - The frame's key expression
    /// * `stage` - Statements encoding the value into `scratch`
    /// * `scratch` - The scratch buffer the value is staged in
    /// * `span` - Span of the emitted write
    ///
    /// # Returns
    ///
    /// A block writing the complete frame into `out`
    pub(super) fn write_staged(
        self,
        key: &impl ToTokens,
        stage: proc_macro2::TokenStream,
        scratch: &syn::Ident,
        span: proc_macro2::Span,
    ) -> proc_macro2::TokenStream {
        // --------------------------------------------------
        // the header, written from the staged length
        // --------------------------------------------------
        let header = match self {
            Self::Separate {
                key_encoder,
                len_encoder,
                ..
            } => {
                let key_write = key_encoder.write(key, span);
                quote_spanned! { span =>
                    #key_write
                    #len_encoder(#scratch.len(), out);
                }
            }
            Self::Fused { keylen_encoder } => {
                let at = helpers::call_site_at(keylen_encoder);
                let mut scratch = scratch.clone();
                scratch.set_span(at);
                quote_spanned! { at => #keylen_encoder((#key, #scratch.len()), out); }
            }
        };
        // --------------------------------------------------
        // stage, header, append
        // --------------------------------------------------
        quote_spanned! { span =>
            {
                #scratch.clear();
                #stage
                #header
                out.extend_from_slice(&#scratch);
            }
        }
    }
}

/// The key encoder of a separate frame header
#[derive(Debug, Clone, Copy)]
pub(super) struct KeyEncoder<'a>(&'a types::XcoderType);

/// [`KeyEncoder`] implementation
impl KeyEncoder<'_> {
    /// Writes a field's key into `out`
    ///
    /// # Arguments
    ///
    /// * `key` - The field's key expression
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

/// The header of a separate frame whose length prefix is fixed-width
///
/// Writes each field's frame directly, with no staging: the length is written up front when the
/// value width is known, else its slot is reserved and back-filled after the value
#[derive(Debug, Clone, Copy)]
pub(super) struct FixedPrefix<'a> {
    /// Encoder writing each field's key
    key_encoder: KeyEncoder<'a>,

    /// The fixed-width length slot following each key
    len: FixedLengthSlot<'a>,
}

/// [`FixedPrefix`] implementation
impl FixedPrefix<'_> {
    /// Writes a frame whose value width is known: key, length, then value
    ///
    /// A value wider than the length slot encodes is a compile error at
    /// [`ValueWidth::declared_at`], emitted in place of the write so every other field still
    /// expands
    ///
    /// # Arguments
    ///
    /// * `key` - The field's key expression
    /// * `value` - The value's known width
    /// * `value_write` - Statements appending the value bytes to `out`
    /// * `span` - Span of the emitted write
    ///
    /// # Returns
    ///
    /// A block writing the complete frame into `out`, checking in debug builds that the length
    /// encoder filled the slot and the value encoder wrote `value.bytes` bytes, or the
    /// `compile_error!` when the slot cannot encode `value.bytes`
    pub(super) fn write_known(
        self,
        key: &impl ToTokens,
        value: ValueWidth,
        value_write: proc_macro2::TokenStream,
        span: proc_macro2::Span,
    ) -> proc_macro2::TokenStream {
        let ValueWidth { bytes, declared_at } = value;
        // --------------------------------------------------
        // a value the slot cannot encode is a compile error
        // --------------------------------------------------
        if !self.len.fits(bytes) {
            let [bytes, width, max_len] = [bytes, self.len.width, self.len.max_len()]
                .map(proc_macro2::Literal::usize_unsuffixed);
            return syn::Error::new(declared_at, err!(LenPrefixTooNarrow(bytes, width, max_len)))
                .to_compile_error();
        }
        // --------------------------------------------------
        // key, length, then value
        // --------------------------------------------------
        let key_write = self.key_encoder.write(key, span);
        let len_write = self.len.write_len(&bytes, span);
        quote_spanned! { span =>
            {
                #key_write
                #len_write
                let __value_start = out.len();
                #value_write
                debug_assert_eq!(
                    out.len() - __value_start,
                    #bytes,
                    "tinyklv: fixed-width value encoder wrote a different byte count than its detected width",
                );
            }
        }
    }

    /// Writes a frame whose value width is unknown: key, then the length slot back-filled after
    /// the value
    ///
    /// # Arguments
    ///
    /// * `key` - The field's key expression
    /// * `value_write` - Statements appending the value bytes to `out`
    /// * `span` - Span of the emitted key write
    ///
    /// # Returns
    ///
    /// The back-filled write from [`FixedLengthSlot::write_backfilled`]
    pub(super) fn write_reserved(
        self,
        key: &impl ToTokens,
        value_write: proc_macro2::TokenStream,
        span: proc_macro2::Span,
    ) -> proc_macro2::TokenStream {
        self.len
            .write_backfilled(self.key_encoder.write(key, span), value_write)
    }
}
