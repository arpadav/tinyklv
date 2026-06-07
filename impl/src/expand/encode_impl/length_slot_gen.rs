//! Fixed-width length-slot emission
//!
//! Generates the complete write for a KLV unit whose body length is unknown
//! until after the body is encoded: write the prefix, reserve a fixed-width
//! length slot, write the value, then back-fill the slot or roll back the whole
//! unit if the value is too large
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::ast::types;

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::quote;

/// Generates a complete fixed-width length-slot write
///
/// Emits generated code that writes a caller-provided prefix, reserves a
/// fixed-width length slot, writes the body, and then back-fills the slot with
/// the final body length. When the slot is too narrow to represent the body,
/// the generated code logs the error and rolls back the entire KLV unit
///
/// # Arguments
///
/// * `prefix_write` - Generated statements that write the field key or sentinel
/// * `len_encoder` - Encoder expression used to write the final body length
/// * `len_width` - Exact byte width reserved for the encoded length slot
/// * `value_write` - Generated statements that append the body bytes to `out`
///
/// # Returns
///
/// A token stream containing the complete generated fixed-slot write sequence
///
/// # Example
///
/// ```rust,ignore
/// let tokens = gen_fixed_length_slot_write(prefix, &len_encoder, 2, value);
/// assert!(!tokens.is_empty());
/// ```
pub(super) fn gen_fixed_length_slot_write(
    prefix_write: proc_macro2::TokenStream,
    len_encoder: &types::SiguledXcoder,
    len_width: usize,
    value_write: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    // --------------------------------------------------
    // fixed slots narrower than usize need overflow handling
    // --------------------------------------------------
    let can_overflow = len_width < core::mem::size_of::<usize>();
    // --------------------------------------------------
    // emit rollback path for potentially overflowing slots
    // --------------------------------------------------
    if can_overflow {
        let max_body_len = 1usize << (8 * len_width);
        let logger = crate::expand::logger::error_logger();
        return quote! {
            {
                let __field_start = out.len();
                #prefix_write
                let __len_pos = out.len();
                out.resize(__len_pos + #len_width, 0u8);
                let __body_start = out.len();
                #value_write
                let __body_len = out.len() - __body_start;
                if __body_len >= #max_body_len {
                    #logger(
                        "tinyklv: KLV item body of {} bytes exceeds its {}-byte length prefix; omitting the item",
                        __body_len,
                        #len_width,
                    );
                    out.truncate(__field_start);
                } else {
                    let __tail = out.len();
                    #len_encoder(__body_len, out);
                    debug_assert_eq!(
                        out.len() - __tail,
                        #len_width,
                        "tinyklv: length encoder wrote a different byte count than its declared fixed `size`",
                    );
                    out.copy_within(__tail.., __len_pos);
                    out.truncate(__tail);
                }
            }
        };
    }
    // --------------------------------------------------
    // emit direct back-fill path for usize-wide slots
    // --------------------------------------------------
    quote! {
        {
            #prefix_write
            let __len_pos = out.len();
            out.resize(__len_pos + #len_width, 0u8);
            let __body_start = out.len();
            #value_write
            let __body_len = out.len() - __body_start;
            let __tail = out.len();
            #len_encoder(__body_len, out);
            debug_assert_eq!(
                out.len() - __tail,
                #len_width,
                "tinyklv: length encoder wrote a different byte count than its declared fixed `size`",
            );
            out.copy_within(__tail.., __len_pos);
            out.truncate(__tail);
        }
    }
}
