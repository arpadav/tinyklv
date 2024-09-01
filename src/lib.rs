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
macro_rules! blank_err {
    () => { winnow::error::ErrMode::Backtrack(winnow::error::ContextError::new()) };
}

#[macro_export]
/// Perform some operation on a parsed value of some defined precision
/// 
/// # Example
/// 
/// ```rust no_run ignore
/// tinyklv::op!(input, f64, * 100.0 - 10.0, 
/// );
/// ```
macro_rules! op {
    ($input:tt, $parser:path, $precision:ty, $($op:tt)+) => { Ok(($parser.parse_next($input)? as $precision) $($op)?) };
}

#[macro_export]
/// Scales a parsed value of some predefined precision
/// 
/// # Example
/// 
/// ```rust no_run ignore
/// tinyklv::scale!(input, tinyklv::codecs::binary::dec::be_u16, f64, KLV_2_PLATFORM_HEADING)
/// );
/// ```
macro_rules! scale {
    ($input:tt, $parser:path, $precision:ty, $scale:tt) => { Ok(($parser.parse_next($input)? as $precision) * $scale) };
}