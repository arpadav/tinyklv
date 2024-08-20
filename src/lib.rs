pub mod prelude;
pub mod defaults;
pub mod primitives;
pub use tinyklv_impl::*;
/// [tinyklv] supports only stream of bytes
pub type Stream<'i> = &'i [u8];