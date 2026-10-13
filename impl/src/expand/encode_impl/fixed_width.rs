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
use crate::ast::types;

// --------------------------------------------------
// constants
// --------------------------------------------------
/// The path segments naming the binary encode module: its own name, and the `encb` alias the
/// docs import it as
///
/// The ASCII encoders in `enc::string` share the native-endian encoders' bare primitive names, so
/// a bare name is only fixed width under one of these
const BINARY_ENCODE_MODULES: [&str; 2] = ["binary", "encb"];

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
/// encode reserves and fixed-length fast paths. `Option<T>` and any non-primitive
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
    if types::is_option(ty) {
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

/// Returns the fixed byte width of a tinyklv built-in binary encoder
///
/// Recognizes only built-in primitive encoder paths: big-endian and little-endian names, `be_` or
/// `le_` followed by a registered primitive name, and native-endian names, the bare primitive
/// name (`encb::u8`) under a [`BINARY_ENCODE_MODULES`] segment. Custom encoders, macros,
/// expressions, ASCII encoders, and lengthed encoders return `None` and use a conservative length
/// plan
///
/// # Arguments
///
/// * `encoder` - Parsed encoder expression: a field's value encoder or the container's key encoder
///
/// # Returns
///
/// `Some(width)` for a recognized built-in fixed-width encoder, or `None` when
/// the encoder cannot be proven to write a stable byte width
pub(super) fn builtin_fixed_encoder_width(encoder: &types::XcoderLike) -> Option<usize> {
    // --------------------------------------------------
    // only path-like built-in encoders can be recognized
    // --------------------------------------------------
    let types::XcoderLike::Path(path) = encoder else {
        return None;
    };
    // --------------------------------------------------
    // extract primitive name from be_ or le_ encoder path
    // --------------------------------------------------
    let mut segments = path.segments.iter().rev();
    let ident = segments.next()?.ident.to_string();
    if let Some(stem) = ident
        .strip_prefix("be_")
        .or_else(|| ident.strip_prefix("le_"))
    {
        return primitive_type_width(stem);
    }
    // --------------------------------------------------
    // a bare primitive name only under the binary module
    // --------------------------------------------------
    if !segments.any(|module| {
        BINARY_ENCODE_MODULES
            .iter()
            .any(|name| module.ident == name)
    }) {
        return None;
    }
    primitive_type_width(&ident)
}

#[cfg(test)]
mod tests {
    use super::{builtin_fixed_encoder_width, primitive_type_width, types};

    #[test]
    /// Verifies a bare primitive encoder name is fixed width only under the binary module
    ///
    /// The ASCII encoders in `enc::string` share the native-endian names but write a variable
    /// width, so a bare name off the binary module is refused, while `be_` and `le_` names are
    /// recognized on any path
    fn builtin_fixed_encoder_width_bare_name_needs_binary_module() {
        // --------------------------------------------------
        // binary encoders resolve to their primitive widths
        // --------------------------------------------------
        for (path, width) in [
            ("encb::u8", 1),
            ("encb::i8", 1),
            ("tinyklv::enc::binary::f64", 8),
            ("tinyklv::codecs::binary::enc::u16", 2),
            ("encb::be_u32", 4),
            ("le_i16", 2),
        ] {
            let encoder = types::XcoderLike::Path(syn::parse_str(path).expect("an encoder path"));
            assert_eq!(
                builtin_fixed_encoder_width(&encoder),
                Some(width),
                "{path} width"
            );
        }
        // --------------------------------------------------
        // a bare name off the binary module is unknown
        // --------------------------------------------------
        for path in [
            "enca::u8",
            "tinyklv::enc::string::i8",
            "tinyklv::codecs::string::enc::u32",
            "u8",
            "encb::u8_from_usize",
        ] {
            let encoder = types::XcoderLike::Path(syn::parse_str(path).expect("an encoder path"));
            assert_eq!(
                builtin_fixed_encoder_width(&encoder),
                None,
                "{path} must not be fixed-width",
            );
        }
    }

    #[test]
    /// Verifies registered primitive widths match Rust's native byte widths
    ///
    /// Confirms that every primitive listed in `primitive_widths!` maps to
    /// `size_of::<T>()` and that intentionally unsupported names are rejected
    /// instead of receiving guessed widths
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
