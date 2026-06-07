//! Field KLV item emission
//!
//! Turns each `#[klv(..)]` field into a complete key-length-value write. The
//! module resolves the encoder call shape, applies optional-field guards, and
//! delegates length-slot bookkeeping to focused helpers
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{
    field_length_plan::{self, ContainerLengthPrefix, FieldLengthPlan},
    length_slot_gen,
};
use crate::{
    ast::{
        attr::{MainContainer, MainField},
        types::{self, XcoderSigil},
    },
    expand::helpers,
};

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::{format_ident, quote, quote_spanned};

/// Generates all field KLV item writes for an `EncodeValue` body
///
/// Walks the parsed container fields and emits one planned KLV item write for
/// each field that carries KLV metadata. Fields without KLV attributes are
/// ignored so ordinary Rust fields remain outside the generated wire output
///
/// # Arguments
///
/// * `input` - Parsed container metadata containing the fields to encode
/// * `key_encoder` - Encoder expression used for each generated field key
/// * `len_encoder` - Encoder expression used for each generated field length
/// * `len_prefix` - Container length-prefix strategy shared by field items
/// * `scratch` - Scratch buffer identifier used by staged value writes
///
/// # Returns
///
/// A token stream containing all generated field item writes in source order
///
/// # Example
///
/// ```rust,ignore
/// let items = gen_klv_items(&container, &key_encoder, &len_encoder, len_prefix, &scratch);
/// assert!(items.is_empty() || !items.to_string().is_empty());
/// ```
pub(super) fn gen_klv_items(
    input: &MainContainer,
    key_encoder: &types::SiguledXcoder,
    len_encoder: &types::SiguledXcoder,
    len_prefix: ContainerLengthPrefix,
    scratch: &syn::Ident,
) -> proc_macro2::TokenStream {
    // --------------------------------------------------
    // emit one planned item for each attributed field
    // --------------------------------------------------
    let items = input
        .data
        .iter()
        .filter_map(|field| gen_klv_item(field, key_encoder, len_encoder, len_prefix, scratch));
    // --------------------------------------------------
    // concatenate generated item writes
    // --------------------------------------------------
    quote! { #(#items)* }
}

/// Generates one field's complete KLV item write when KLV metadata exists
///
/// Resolves the field's key, value encoder call, optional-value binding shape,
/// and length plan before delegating to the lower-level planned write helper
/// Fields without KLV metadata return `None`, leaving them out of the generated
/// value body entirely
///
/// # Arguments
///
/// * `field` - Parsed field descriptor to convert into a KLV item write
/// * `key_encoder` - Encoder expression used for the field key
/// * `len_encoder` - Encoder expression used for the field length
/// * `len_prefix` - Container length-prefix strategy used for this field
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
/// let item = gen_klv_item(&field, &key_encoder, &len_encoder, len_prefix, &scratch);
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
    key_encoder: &types::SiguledXcoder,
    len_encoder: &types::SiguledXcoder,
    len_prefix: ContainerLengthPrefix,
    scratch: &syn::Ident,
) -> Option<proc_macro2::TokenStream> {
    // --------------------------------------------------
    // skip fields without KLV metadata
    // --------------------------------------------------
    let attrs = field.attrs.as_ref()?;
    #[allow(
        clippy::unwrap_used,
        reason = "`gen_encode_impl` only calls encode generation when every field encoder exists"
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
    // choose the field length write strategy
    // --------------------------------------------------
    let length_plan = field_length_plan::field_length_plan(
        attrs.fallback_enc,
        value_encoder,
        field.ty,
        len_prefix,
        attrs.size,
    );
    // --------------------------------------------------
    // emit the concrete planned KLV item write
    // --------------------------------------------------
    let item_write = gen_planned_item_write(PlannedItemWrite {
        key: &attrs.key,
        key_encoder,
        len_encoder,
        value_encoder_call,
        value_argument,
        length_plan,
        scratch,
        span,
    });
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
    /// Field key literal emitted before the item length and value bytes
    key: &'a syn::Lit,

    /// Encoder expression used to write the field key
    key_encoder: &'a types::SiguledXcoder,

    /// Encoder expression used to write the field length
    len_encoder: &'a types::SiguledXcoder,

    /// Callable token expression used to encode the field value bytes
    value_encoder_call: proc_macro2::TokenStream,

    /// Argument token expression passed to the value encoder callable
    value_argument: proc_macro2::TokenStream,

    /// Selected strategy for writing or back-filling the field length
    length_plan: FieldLengthPlan,

    /// Scratch buffer identifier used by staged value writes
    scratch: &'a syn::Ident,

    /// Source span used to attach diagnostics to generated field code
    span: proc_macro2::Span,
}

/// Emits the complete field write selected by [`FieldLengthPlan`]
///
/// Converts a fully planned field item into concrete generated statements
/// Known-width values write their key, length, and value directly; reserved
/// slots delegate fixed-width back-fill handling; staged values encode into the
/// shared scratch buffer before writing the final KLV item
///
/// # Arguments
///
/// * `parts` - Planned semantic inputs for one field item write
///
/// # Returns
///
/// A token stream containing the generated statements for one complete field
/// item write
///
/// # Example
///
/// ```rust,ignore
/// let tokens = gen_planned_item_write(parts);
/// assert!(!tokens.is_empty());
/// ```
fn gen_planned_item_write(parts: PlannedItemWrite<'_>) -> proc_macro2::TokenStream {
    // --------------------------------------------------
    // unpack the planned write inputs
    // --------------------------------------------------
    let PlannedItemWrite {
        key,
        key_encoder,
        len_encoder,
        value_encoder_call,
        value_argument,
        length_plan,
        scratch,
        span,
    } = parts;
    // --------------------------------------------------
    // emit the write shape selected by the length plan
    // --------------------------------------------------
    match length_plan {
        FieldLengthPlan::KnownValueWidth { value_width } => quote_spanned! { span =>
            {
                #key_encoder(#key, out);
                #len_encoder(#value_width, out);
                let __value_start = out.len();
                #value_encoder_call(#value_argument, out);
                debug_assert_eq!(
                    out.len() - __value_start,
                    #value_width,
                    "tinyklv: fixed-width value encoder wrote a different byte count than its detected width",
                );
            }
        },
        FieldLengthPlan::ReservedLengthSlot { len_width } => {
            let prefix_write = quote_spanned! { span => #key_encoder(#key, out); };
            let value_write = quote_spanned! { span => #value_encoder_call(#value_argument, out); };
            length_slot_gen::gen_fixed_length_slot_write(
                prefix_write,
                len_encoder,
                len_width,
                value_write,
            )
        }
        FieldLengthPlan::StagedValue => quote_spanned! { span =>
            {
                #scratch.clear();
                #value_encoder_call(#value_argument, &mut #scratch);
                #key_encoder(#key, out);
                #len_encoder(#scratch.len(), out);
                out.extend_from_slice(&#scratch);
            }
        },
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
