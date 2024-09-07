#![doc = include_str!("../README.md")]
pub mod _tutorial;
pub mod prelude;
pub mod codecs;
pub use codecs::*;
pub mod reexport {
    pub use winnow;
}
pub use tinyklv_impl::*;

#[macro_export]
/// Returns a blank context error: usually used for reserved values.
/// 
/// It is not recommended to use this unless a `None` value has to be
/// returned upon parsing values.
macro_rules! err {
    () => { winnow::error::ErrMode::Backtrack(winnow::error::ContextError::new()) };
}

// #[macro_export]
// /// Perform some operation on a parsed value of some defined precision
// /// 
// /// # Example
// /// 
// /// ```rust no_run ignore
// /// tinyklv::op!(input, parser, f64, * 100.0, - 10.0);
// /// // expands to:
// /// let _ = (parser.parse_next(input)? * 100.0) - 10.0;
// /// 
// /// tinyklv::op!(input, parser, f64, * 100.0, - 10.0, + 12.0, / 2.0, + 1.0);
// /// // expands to:
// /// let _ = (((parser.parse_next(input)? * 100.0) - 10.0 + 12.0) / 2.0) + 1.0;
// /// ```
// macro_rules! op {
//     // Base case: single operation
//     ($input:tt, $parser:path, $precision:ty, $op:tt $val:expr) => {
//         $parser.parse_next($input)? $op $val
//     };

//     // Recursive case: multiple operations
//     ($input:tt, $parser:path, $precision:ty, $op1:tt $val1:expr, $($op2:tt $val2:expr),*) => {
//         $crate::op!(@apply $input, $parser, $precision, $op1 $val1, $($op2 $val2),*)
//     };

//     // Helper to apply remaining operations
//     (@apply $input:tt, $parser:path, $precision:ty, $op1:tt $val1:expr) => {
//         $parser.parse_next($input)? $op1 $val1
//     };

//     (@apply $input:tt, $parser:path, $precision:ty, $op1:tt $val1:expr, $($op2:tt $val2:expr),*) => {
//         $crate::op!(@apply $input, $parser, $precision, ($parser.parse_next($input)? as $precision $op1 $val1) $($op2 $val2),*)
//     };
// }
// Example Usage
// tinyklv::op!(input, parser, f64, * 100.0, - 10.0, + 12.0, / 2.0, + 1.0);


#[macro_export]
/// Scales a parsed value of some predefined precision
/// 
/// # Example
/// 
/// ```rust no_run ignore
/// tinyklv::scale!(tinyklv::codecs::binary::dec::be_u16, f64, KLV_2_PLATFORM_HEADING)(input)
/// // OR
/// #[klv(dec = tinyklv::scale!(tinyklv::codecs::binary::dec::be_u16, f64, KLV_2_PLATFORM_HEADING))]
/// ```
macro_rules! scale {
    ($parser:path, $precision:ty, $scale:tt $(,)*) => {
        |input| -> winnow::PResult<$precision> {
            Ok(($parser.parse_next(input)? as $precision) * $scale)
        }
    };
}

#[macro_export]
/// Sets precision of a parsed value
/// 
/// # Example
/// 
/// ```rust no_run ignore
/// tinyklv::cast!(tinyklv::codecs::binary::dec::be_u16, f64)(input)
/// // OR
/// #[klv(dec = tinyklv::cast!(tinyklv::codecs::binary::dec::be_u16, f64))]
/// ```
macro_rules! cast {
    ($parser:expr, $precision:ty $(,)*) => {
        |input| -> winnow::PResult<$precision> {
            Ok($parser.parse_next(input)? as $precision)
        }
    };
}