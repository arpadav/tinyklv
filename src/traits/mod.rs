// --------------------------------------------------
// mods
// --------------------------------------------------
mod dec;
mod enc;
mod types;

// --------------------------------------------------
// local
// --------------------------------------------------
pub use dec::*; // BreakConditionType, // <-- todo: move this
pub use enc::*;
pub use types::*;

#[allow(dead_code)]
/// A fixed length decoder function signature
///
/// **This type is for documentation purposes only**
///
/// `fn <name>(input: &mut S) -> tinyklv::Result<Self>`
type FixedDecodeSignature = ();

#[allow(dead_code)]
/// A variable length decoder function signature
///
/// **This type is for documentation purposes only**
///
/// `fn <name>(len: usize) -> impl Fn(&mut S) -> tinyklv::Result<Self>`
type VariableDecodeSignature = ();

/// A type
pub enum Length {
    /// An explicit length of fixed size, in bytes.
    ///
    /// For example, an unsigned 16 bit integer is [`Length::Fixed(2)`](Length::Fixed)
    ///
    /// Decoding function signatures for [`Length::Fixed`] types are required to be
    /// [`FixedDecodeSignature`]
    Fixed(usize),

    /// An implied length determined during decoding.
    ///
    /// This varies from [`Length::Fixed`] and [`Length::Variable`], in the
    /// sense that:
    ///
    /// 1. Same function signature as [`Length::Fixed`], but no explicit length
    ///   checking can be performed.
    /// 2. Different function signature as [`Length::Variable`]
    ///
    /// Decoding function signatures for [`Length::Implicit`] types are required to be
    /// [`FixedDecodeSignature`]
    Implicit,

    /// An explicitly defined variable length.
    ///
    /// For example, a string
    ///
    /// Decoding function signatures for [`Length::Variable`] types are required to be
    /// [`VariableDecodeSignature`]
    Variable,
}

// pub trait TinyklvDoc {
//     /// This is a doc comment
//     fn allow_unimplemented_encode();
// }
