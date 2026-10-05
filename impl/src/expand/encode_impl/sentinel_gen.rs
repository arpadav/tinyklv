//! `EncodeFrame` impl emission
//!
//! Generates the sentinel-framed encoder for containers that declare a
//! sentinel. Fixed-width frame lengths back-fill the shared fixed-width length slot;
//! variable-width frame lengths stage the whole body first
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{field_length_plan::ContainerLengthPrefix, length_slot_gen::FixedLengthSlot};
use crate::ast::types;

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::quote;

// --------------------------------------------------
// constants
// --------------------------------------------------
/// Extra reserve space for a variable-width BER frame length prefix
const BER_LEN_PREFIX_BYTES: usize = 8;

/// Generates the `EncodeFrame` impl of a container that declares a sentinel
///
/// The generated `encode_frame` writes the sentinel, the frame length with the separate length
/// encoder, and the encoded value body
///
/// # Arguments
///
/// * `name` - The container ident the impl is generated for
/// * `generics` - Generic parameters from the original struct definition
/// * `sentinel` - Literal sentinel bytes written before the frame length
/// * `len_encoder` - Encoder expression used to write the frame length
/// * `len_prefix` - The length prefix's shape, choosing a back-filled slot or a staged body
/// * `value_reserve_bytes` - Reserve hint for the encoded value body
///
/// # Returns
///
/// A token stream containing the generated `EncodeFrame` implementation
pub(super) fn gen_encode_frame_impl(
    name: &syn::Ident,
    generics: &syn::Generics,
    sentinel: &syn::Lit,
    len_encoder: &types::XcoderType,
    len_prefix: ContainerLengthPrefix,
    value_reserve_bytes: usize,
) -> proc_macro2::TokenStream {
    // --------------------------------------------------
    // build the container impl shape and frame body
    // --------------------------------------------------
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let frame_write = gen_frame_write(sentinel, len_encoder, len_prefix, value_reserve_bytes);
    // --------------------------------------------------
    // assemble the trait impl
    // --------------------------------------------------
    quote! {
        #[automatically_derived]
        impl #impl_generics ::tinyklv::traits::EncodeFrame for #name #ty_generics #where_clause {
            fn encode_frame(&self, out: &mut ::std::vec::Vec<u8>) {
                #frame_write
            }
        }
    }
}

/// Generates the complete write performed inside `encode_frame`
///
/// Selects between the fixed-length-slot path and the variable-length staging
/// path for a framed container. Fixed-width frames reserve a length slot and
/// back-fill it after writing the value, while variable-width frames encode the
/// body into a temporary buffer so the final length is known before emission
///
/// # Arguments
///
/// * `sentinel` - Literal sentinel bytes written before the frame length
/// * `len_encoder` - Encoder expression used to write the frame length
/// * `len_prefix` - Container length-prefix strategy for the frame wrapper
/// * `value_reserve_bytes` - Reserve hint for the encoded value body
///
/// # Returns
///
/// A token stream containing the generated statements that write one complete
/// encoded frame into `out`
///
/// # Example
///
/// ```rust,ignore
/// let tokens = gen_frame_write(&sentinel, &len_encoder, len_prefix, reserve);
/// assert!(!tokens.is_empty());
/// ```
fn gen_frame_write(
    sentinel: &syn::Lit,
    len_encoder: &types::XcoderType,
    len_prefix: ContainerLengthPrefix,
    value_reserve_bytes: usize,
) -> proc_macro2::TokenStream {
    // --------------------------------------------------
    // select fixed-slot or staged variable-length frame writing
    // --------------------------------------------------
    match len_prefix {
        ContainerLengthPrefix::Fixed { width } => {
            // --------------------------------------------------
            // write sentinel and reserve a fixed frame length slot
            // --------------------------------------------------
            let sentinel_write = quote! {
                out.reserve(#value_reserve_bytes + 2 * #width + #sentinel.len());
                out.extend_from_slice(#sentinel);
            };
            let value_write = quote! { self.encode_value(out); };
            // --------------------------------------------------
            // delegate fixed-slot back-fill bookkeeping
            // --------------------------------------------------
            FixedLengthSlot { len_encoder, width }.write_backfilled(sentinel_write, value_write)
        }
        ContainerLengthPrefix::Variable => quote! {
            {
                let mut __body: ::std::vec::Vec<u8> = ::std::vec::Vec::new();
                self.encode_value(&mut __body);
                out.reserve(__body.len() + #sentinel.len() + #BER_LEN_PREFIX_BYTES);
                out.extend_from_slice(#sentinel);
                #len_encoder(__body.len(), out);
                out.extend_from_slice(&__body);
            }
        },
    }
}
