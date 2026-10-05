//! Field KLV item emission
//!
//! Turns each `#[klv(..)]` field into a complete key-length-value write. The
//! module resolves the encoder call shape, picks the write the container's framing
//! selects for the field, applies optional-field guards, and hands the header write to
//! the container framing
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{
    field_length_plan::{FieldLengthPlan, ValueWidth},
    fixed_width,
    framing::EncodeFraming,
};
use crate::{
    ast::{
        attr::{MainContainer, MainField, size::SizeSpec},
        types::{self, XcoderSigil},
    },
    expand::helpers,
};

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned;

/// Generates all field KLV item writes for an `EncodeValue` body
///
/// Walks the parsed container fields and emits one planned KLV item write for
/// each field that carries KLV metadata. Fields without KLV attributes are
/// ignored so ordinary Rust fields remain outside the generated wire output
///
/// # Arguments
///
/// * `input` - Parsed container metadata containing the fields to encode
/// * `framing` - The frame header encoder(s) written around each field value
/// * `scratch` - Scratch buffer identifier used by staged value writes
///
/// # Returns
///
/// A token stream containing all generated field item writes in source order
///
/// # Example
///
/// ```rust,ignore
/// let items = gen_klv_items(&container, framing, &scratch);
/// assert!(items.is_empty() || !items.to_string().is_empty());
/// ```
pub(super) fn gen_klv_items(
    input: &MainContainer,
    framing: EncodeFraming<'_>,
    scratch: &syn::Ident,
) -> proc_macro2::TokenStream {
    // --------------------------------------------------
    // emit one planned item for each attributed field
    // --------------------------------------------------
    let items = input
        .data
        .iter()
        .filter_map(|field| gen_klv_item(field, framing, scratch));
    // --------------------------------------------------
    // concatenate generated item writes
    // --------------------------------------------------
    quote! { #(#items)* }
}

/// Generates one field's complete KLV item write when KLV metadata exists
///
/// Resolves the field's key, value encoder call, optional-value binding shape,
/// and planned item write, then emits that write as the container framing selects.
/// Fields without KLV metadata return `None`, leaving them out of the generated
/// value body entirely
///
/// # Arguments
///
/// * `field` - Parsed field descriptor to convert into a KLV item write
/// * `framing` - The frame header encoder(s) written around the field value
/// * `scratch` - Scratch buffer identifier used by staged value writes
///
/// # Returns
///
/// `Some(tokens)` with a complete generated KLV item write for attributed
/// fields, or `None` for fields that do not participate in encoding
///
/// # Example
///
/// ```rust,ignore
/// let item = gen_klv_item(&field, framing, &scratch);
/// assert!(item.is_none() || !item.unwrap().is_empty());
/// ```
///
/// # Safety
///
/// This helper unwraps `attrs.enc` after the derive pipeline has validated that
/// every field entering encode generation has a value encoder. If that invariant
/// is broken by a future caller, generation will panic instead of producing an
/// invalid encoder
fn gen_klv_item(
    field: &MainField,
    framing: EncodeFraming<'_>,
    scratch: &syn::Ident,
) -> Option<proc_macro2::TokenStream> {
    // --------------------------------------------------
    // skip fields without KLV metadata
    // --------------------------------------------------
    let attrs = field.attrs.as_ref()?;
    #[allow(
        clippy::unwrap_used,
        reason = "`gen_encode_impl` returns before item generation unless every field has an encoder"
    )]
    let value_encoder = attrs.enc.as_ref().unwrap();
    // --------------------------------------------------
    // prepare field-local token inputs
    // --------------------------------------------------
    let span = field.name.span();
    let value_binding = format_ident!("__val");
    let value_encoder_call = value_encoder_tokens(attrs.fallback_enc, field.ty, value_encoder);
    let value_argument =
        value_argument_tokens(&field.name, field.ty, value_encoder, &value_binding);
    // --------------------------------------------------
    // select required or optional encoder argument shape
    // --------------------------------------------------
    let value_argument = if value_argument.is_optional {
        value_argument.optional
    } else {
        value_argument.required
    };
    // --------------------------------------------------
    // emit the item write the container framing selects
    // --------------------------------------------------
    let item_write = PlannedItemWrite {
        key: &attrs.key,
        fallback_enc: attrs.fallback_enc,
        value_encoder,
        ty: field.ty,
        size: attrs.size,
        value_encoder_call,
        value_argument,
        scratch,
        span,
    }
    .emit(framing);
    // --------------------------------------------------
    // wrap optional fields with their `Some` guard
    // --------------------------------------------------
    Some(wrap_optional_field(
        &field.name,
        helpers::is_option(field.ty),
        &value_binding,
        item_write,
        span,
    ))
}

/// Value arguments for required and optional field forms
struct ValueArgumentTokens {
    /// Token expression passed to the value encoder for required fields
    required: proc_macro2::TokenStream,

    /// Token expression passed to the value encoder inside an optional guard
    optional: proc_macro2::TokenStream,

    /// Whether the source field type is `Option<T>` and needs optional handling
    is_optional: bool,
}

/// Builds the encoder path used for a field value
///
/// Chooses between the fallback `EncodeValue::encode_value` method for the
/// field type and the explicit encoder expression declared in the field
/// attributes. Optional field types are unwrapped for fallback encoding so the
/// generated call targets the contained value type
///
/// # Arguments
///
/// * `fallback_enc` - Whether to use the field type's fallback trait encoder
/// * `ty` - Parsed field type used when fallback encoding is selected
/// * `value_encoder` - Explicit parsed encoder expression from field metadata
///
/// # Returns
///
/// A token stream containing the encoder callable used for the field value
///
/// # Example
///
/// ```rust,ignore
/// let encoder = value_encoder_tokens(false, &ty, &value_encoder);
/// assert!(!encoder.is_empty());
/// ```
fn value_encoder_tokens(
    fallback_enc: bool,
    ty: &syn::Type,
    value_encoder: &types::SiguledXcoder,
) -> proc_macro2::TokenStream {
    // --------------------------------------------------
    // resolve fallback trait encoder or explicit encoder path
    // --------------------------------------------------
    if fallback_enc {
        let ty = helpers::unwrap_option_type(ty).unwrap_or(ty);
        quote! { <#ty as ::tinyklv::traits::EncodeValue>::encode_value }
    } else {
        let inner = &value_encoder.inner;
        quote! { #inner }
    }
}

/// Shapes the field value argument according to the encoder sigil
///
/// Builds both the required-field argument expression and the optional-field
/// argument expression because the optional guard introduces a local binding for
/// the inner value. The selected sigil controls whether the encoder receives a
/// reference, an `EncodeAs` projection, or a dereferenced value
///
/// # Arguments
///
/// * `name` - Field identifier used to access `self.field` in generated code
/// * `ty` - Parsed field type used to detect optional fields
/// * `value_encoder` - Parsed value encoder whose sigil controls call shape
/// * `value_binding` - Local binding name introduced for optional inner values
///
/// # Returns
///
/// Required and optional token expressions plus a flag indicating which one
/// should be selected for the current field type
///
/// # Example
///
/// ```rust,ignore
/// let args = value_argument_tokens(&name, &ty, &value_encoder, &value_binding);
/// assert!(args.is_optional || !args.required.is_empty());
/// ```
fn value_argument_tokens(
    name: &syn::Ident,
    ty: &syn::Type,
    value_encoder: &types::SiguledXcoder,
    value_binding: &syn::Ident,
) -> ValueArgumentTokens {
    // --------------------------------------------------
    // build required and optional argument forms for the sigil
    // --------------------------------------------------
    let span = name.span();
    let (required, optional) = match value_encoder.sigil {
        XcoderSigil::None => (
            quote_spanned! { span => &self.#name },
            quote_spanned! { span => #value_binding },
        ),
        XcoderSigil::Ref => (
            quote_spanned! { span => ::tinyklv::traits::EncodeAs::encode_as(&self.#name) },
            quote_spanned! { span => ::tinyklv::traits::EncodeAs::encode_as(#value_binding) },
        ),
        XcoderSigil::Deref => (
            quote_spanned! { span => self.#name },
            quote_spanned! { span => *#value_binding },
        ),
    };
    ValueArgumentTokens {
        required,
        optional,
        is_optional: helpers::is_option(ty),
    }
}

/// All semantic inputs required to emit a complete field item write
struct PlannedItemWrite<'a> {
    /// Field key literal emitted in the frame header before the value bytes
    key: &'a syn::Lit,

    /// Whether the value uses the fallback `EncodeValue` encoder
    fallback_enc: bool,

    /// Parsed value encoder, whose built-in width may fix the value length
    value_encoder: &'a types::SiguledXcoder,

    /// The field's declared type, whose primitive width may fix the value length
    ty: &'a syn::Type,

    /// The field's declared value size, whose exact width fixes the value length
    size: Option<SizeSpec>,

    /// Callable token expression used to encode the field value bytes
    value_encoder_call: proc_macro2::TokenStream,

    /// Argument token expression passed to the value encoder callable
    value_argument: proc_macro2::TokenStream,

    /// Scratch buffer identifier used by staged value writes
    scratch: &'a syn::Ident,

    /// Source span used to attach diagnostics to generated field code
    span: proc_macro2::Span,
}

/// [`PlannedItemWrite`] implementation
impl PlannedItemWrite<'_> {
    /// Picks the field write: staged, known or reserved
    ///
    /// A fused `keylen` encoder, or a variable-width length prefix, stages the value through
    /// [`EncodeFraming::write_staged`]. Under a fixed-width length prefix
    /// ([`EncodeFraming::fixed_prefix`]) the [`FieldLengthPlan`] picks a known-width or a
    /// reserved-slot write, and the [`FixedPrefix`](super::framing::FixedPrefix) writes it
    ///
    /// # Arguments
    ///
    /// * `framing` - The frame header encoder(s) written around the value
    ///
    /// # Returns
    ///
    /// A token stream containing the generated statements for one complete field item write
    fn emit(&self, framing: EncodeFraming<'_>) -> proc_macro2::TokenStream {
        let Some(prefix) = framing.fixed_prefix() else {
            return self.write_staged(framing);
        };
        let Self { key, span, .. } = *self;
        let value_write = self.value_write(&quote_spanned! { span => out });
        match self.length_plan() {
            FieldLengthPlan::KnownValueWidth(value) => {
                prefix.write_known(key, value, value_write, span)
            }
            FieldLengthPlan::ReservedLengthSlot => prefix.write_reserved(key, value_write, span),
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
    fn length_plan(&self) -> FieldLengthPlan {
        // --------------------------------------------------
        // explicit exact field sizes can be written immediately
        // --------------------------------------------------
        if let Some(size) = self.size
            && let Some(bytes) = size.exact_width()
        {
            return FieldLengthPlan::KnownValueWidth(ValueWidth {
                bytes,
                declared_at: size.span(),
            });
        }
        // --------------------------------------------------
        // prove fixed width from primitive type and built-in encoder
        // --------------------------------------------------
        if !self.fallback_enc {
            let inner = helpers::unwrap_option_type(self.ty).unwrap_or(self.ty);
            if let (Some(type_width), Some(encoder_width)) = (
                fixed_width::fixed_value_type_width(inner),
                fixed_width::builtin_fixed_encoder_width(&self.value_encoder.inner),
            ) && type_width == encoder_width
            {
                return FieldLengthPlan::KnownValueWidth(ValueWidth {
                    bytes: type_width,
                    declared_at: self.value_encoder.inner.span(),
                });
            }
        }
        // --------------------------------------------------
        // otherwise reserve the container's fixed length slot
        // --------------------------------------------------
        FieldLengthPlan::ReservedLengthSlot
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
        let Self {
            value_encoder_call,
            value_argument,
            span,
            ..
        } = self;
        quote_spanned! { *span => #value_encoder_call(#value_argument, #dst); }
    }

    /// The field value staged in the scratch buffer, then written as one frame
    ///
    /// # Arguments
    ///
    /// * `framing` - The frame header encoder(s) written around the staged value
    ///
    /// # Returns
    ///
    /// The staged frame write from [`EncodeFraming::write_staged`]
    fn write_staged(&self, framing: EncodeFraming<'_>) -> proc_macro2::TokenStream {
        let Self { scratch, span, .. } = *self;
        let stage = self.value_write(&quote_spanned! { span => &mut #scratch });
        framing.write_staged(self.key, stage, scratch, span)
    }
}

/// Wraps optional fields in a `Some` guard and writes required fields directly
///
/// Optional fields emit no KLV item when their value is `None`. Required fields
/// use the planned item write unchanged, preserving the same generated span so
/// downstream compiler diagnostics point back to the source field
///
/// # Arguments
///
/// * `name` - Field identifier used to access the optional field on `self`
/// * `is_optional` - Whether the source field type is `Option<T>`
/// * `value_binding` - Local binding name for the optional inner value
/// * `item_write` - Generated item write to execute when the field is present
/// * `span` - Source span attached to the generated guard or write expression
///
/// # Returns
///
/// A token stream containing either the guarded optional write or the original
/// required-field write
///
/// # Example
///
/// ```rust,ignore
/// let wrapped = wrap_optional_field(&name, true, &value_binding, item_write, span);
/// assert!(!wrapped.is_empty());
/// ```
fn wrap_optional_field(
    name: &syn::Ident,
    is_optional: bool,
    value_binding: &syn::Ident,
    item_write: proc_macro2::TokenStream,
    span: proc_macro2::Span,
) -> proc_macro2::TokenStream {
    // --------------------------------------------------
    // guard optional fields and write required fields directly
    // --------------------------------------------------
    if is_optional {
        quote_spanned! { span =>
            if let ::core::option::Option::Some(ref #value_binding) = self.#name {
                #item_write
            }
        }
    } else {
        quote_spanned! { span => #item_write }
    }
}
