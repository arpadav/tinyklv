//! Compile-time byte-width resolution for fixed-width encode paths
//!
//! A single primitive-width registry feeds two encode-only lookups: by field
//! type, used for reserve estimates and fixed-value fast paths, and by built-in
//! encoder path, used to prove a value encoder writes that same fixed width
//! Neither lookup guesses from arbitrary paths
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::{ast::types, expand::helpers};

/// Generates [`primitive_type_width`] from the single fixed-width primitive list
macro_rules! primitive_widths {
    ($($prim:ident),+ $(,)?) => {
        /// Returns the encoded byte width of a fixed-width primitive type name
        ///
        /// Looks up only primitive names registered in this module, keeping
        /// compile-time width inference conservative for generated encoders
        /// Unknown names return `None` so callers can fall back to staged or
        /// reserved length handling instead of guessing
        ///
        /// # Arguments
        ///
        /// * `name` - Primitive Rust type name to resolve without path segments
        ///
        /// # Returns
        ///
        /// `Some(width)` for a registered fixed-width primitive, or `None` for
        /// every unsupported or non-primitive name
        ///
        /// # Example
        ///
        /// ```rust,ignore
        /// assert_eq!(primitive_type_width("u16"), Some(2));
        /// assert_eq!(primitive_type_width("String"), None);
        /// ```
        fn primitive_type_width(name: &str) -> Option<usize> {
            match name {
                $(stringify!($prim) => Some(::core::mem::size_of::<$prim>()),)+
                _ => None,
            }
        }
    };
}
primitive_widths!(u8, i8, u16, i16, u32, i32, f32, u64, i64, f64, u128, i128);

/// Returns the encoded byte width of a fixed-width primitive field type
///
/// Resolves direct primitive field types to their byte width for generated
/// reserve hints and fixed-length fast paths. `Option<T>` and any non-primitive
/// return `None`; optional fields are not a fixed number of bytes on the wire
/// because `None` emits no KLV item
///
/// # Arguments
///
/// * `ty` - Parsed Rust field type to classify for fixed-width encoding
///
/// # Returns
///
/// `Some(width)` when the type is a registered fixed-width primitive, or `None`
/// when the type is optional, indirect, generic, or otherwise unsupported
///
/// # Example
///
/// ```rust,ignore
/// let ty: syn::Type = syn::parse_quote!(u32);
/// assert_eq!(fixed_value_type_width(&ty), Some(4));
/// ```
pub(super) fn fixed_value_type_width(ty: &syn::Type) -> Option<usize> {
    // --------------------------------------------------
    // optional fields do not emit a fixed number of bytes
    // --------------------------------------------------
    if helpers::is_option(ty) {
        return None;
    }
    // --------------------------------------------------
    // only direct primitive type paths are fixed width
    // --------------------------------------------------
    let syn::Type::Path(path) = ty else {
        return None;
    };
    primitive_type_width(&path.path.get_ident()?.to_string())
}

/// Returns the fixed byte width of a tinyklv built-in binary value encoder
///
/// Recognizes only built-in big-endian and little-endian primitive encoder
/// paths whose names start with `be_` or `le_` followed by a registered
/// primitive name. Custom encoders, macros, expressions, ASCII encoders, and
/// lengthed encoders return `None` and use a conservative length plan
///
/// # Arguments
///
/// * `value_encoder` - Parsed value encoder expression from field metadata
///
/// # Returns
///
/// `Some(width)` for a recognized built-in fixed-width encoder, or `None` when
/// the encoder cannot be proven to write a stable byte width
///
/// # Example
///
/// ```rust,ignore
/// let width = builtin_fixed_encoder_width(&value_encoder);
/// assert!(width.is_none() || width.unwrap() > 0);
/// ```
pub(super) fn builtin_fixed_encoder_width(value_encoder: &types::SiguledXcoder) -> Option<usize> {
    // --------------------------------------------------
    // only path-like built-in encoders can be recognized
    // --------------------------------------------------
    let types::XcoderLike::Path(path) = &value_encoder.inner else {
        return None;
    };
    // --------------------------------------------------
    // extract primitive name from be_ or le_ encoder path
    // --------------------------------------------------
    let ident = path.segments.last()?.ident.to_string();
    let stem = ident
        .strip_prefix("be_")
        .or_else(|| ident.strip_prefix("le_"))?;
    primitive_type_width(stem)
}

#[cfg(test)]
mod tests {
    use super::primitive_type_width;

    #[test]
    /// Verifies registered primitive widths match Rust's native byte widths
    ///
    /// Confirms that every primitive listed in `primitive_widths!` maps to
    /// `size_of::<T>()` and that intentionally unsupported names are rejected
    /// instead of receiving guessed widths
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// primitive_type_width_matches_size_of();
    /// ```
    fn primitive_type_width_matches_size_of() {
        // --------------------------------------------------
        // primitives resolve to their platform byte widths
        // --------------------------------------------------
        for (name, width) in [
            ("u8", size_of::<u8>()),
            ("i8", size_of::<i8>()),
            ("u16", size_of::<u16>()),
            ("i16", size_of::<i16>()),
            ("u32", size_of::<u32>()),
            ("i32", size_of::<i32>()),
            ("f32", size_of::<f32>()),
            ("u64", size_of::<u64>()),
            ("i64", size_of::<i64>()),
            ("f64", size_of::<f64>()),
            ("u128", size_of::<u128>()),
            ("i128", size_of::<i128>()),
        ] {
            assert_eq!(primitive_type_width(name), Some(width), "{name} width");
        }
        // --------------------------------------------------
        // non-registered names are deliberately unknown
        // --------------------------------------------------
        for name in ["usize", "isize", "bool", "String", "u256"] {
            assert_eq!(
                primitive_type_width(name),
                None,
                "{name} must not be fixed-width",
            );
        }
    }
}
