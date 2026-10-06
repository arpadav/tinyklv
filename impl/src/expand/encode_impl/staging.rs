//! The staged write: a value encoded into a buffer before its frame header, so the header can carry
//! the value's length
//!
//! Every staged write has one shape: ready the buffer, encode the value into it, write the header
//! from the staged length, then append the staged bytes to `out`
//!
//! Author: aav
// --------------------------------------------------
// external
// --------------------------------------------------
use quote::{format_ident, quote, quote_spanned};

/// The buffer a value is staged in before its header is written
#[derive(Debug, Clone, Copy)]
pub(super) enum Staging {
    /// `__scratch`, `encode_value`'s buffer: declared once per call and cleared before each value
    Scratch,
    /// `__body`, a buffer declared for this one write
    Fresh,
}

/// [`Staging`] implementation
impl Staging {
    /// The buffer's binding
    ///
    /// # Returns
    ///
    /// The identifier the staged value is encoded into
    pub(super) fn buffer(self) -> syn::Ident {
        match self {
            Self::Scratch => format_ident!("__scratch"),
            Self::Fresh => format_ident!("__body"),
        }
    }

    /// Declares the buffer, empty
    ///
    /// # Returns
    ///
    /// The `let mut` binding of an empty `Vec<u8>`
    pub(super) fn declare(self) -> proc_macro2::TokenStream {
        let buffer = self.buffer();
        quote! { let mut #buffer: ::std::vec::Vec<u8> = ::std::vec::Vec::new(); }
    }

    /// Writes one staged frame: ready the buffer, stage the value, write the header, then append
    ///
    /// # Arguments
    ///
    /// * `header` - The statements writing the frame header from [`Self::buffer`]'s length
    /// * `value_write` - Builds the statements encoding the value into the buffer it is given
    /// * `span` - Span of the emitted write
    ///
    /// # Returns
    ///
    /// A block writing the complete frame into `out`
    pub(super) fn write(
        self,
        header: proc_macro2::TokenStream,
        value_write: impl FnOnce(&proc_macro2::TokenStream) -> proc_macro2::TokenStream,
        span: proc_macro2::Span,
    ) -> proc_macro2::TokenStream {
        let buffer = self.buffer();
        // --------------------------------------------------
        // a scratch buffer is cleared, a fresh one declared
        // --------------------------------------------------
        let ready = match self {
            Self::Scratch => quote_spanned! { span => #buffer.clear(); },
            Self::Fresh => self.declare(),
        };
        let stage = value_write(&quote_spanned! { span => &mut #buffer });
        // --------------------------------------------------
        // stage, header, append
        // --------------------------------------------------
        quote_spanned! { span =>
            {
                #ready
                #stage
                #header
                out.extend_from_slice(&#buffer);
            }
        }
    }
}
