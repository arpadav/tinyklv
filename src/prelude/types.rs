//! Primitive types for the [`crate::prelude`]

/// A trait to represent the encoded output.
/// 
/// * This trait defines a type which must be iterable of some Element `T` (derives from [`HasElement`])
/// * This trait defines a type which must must be transcodable (derives from [`TranscodableIterable`])
pub trait EncodedOutput: HasElement + TranscodableIterable<<Self as HasElement>::Element> {}
impl<O> EncodedOutput for O where O: HasElement + TranscodableIterable<<O as HasElement>::Element> {}

/// A trait to represent a transcodable iterable
/// 
/// This is mainly used for encoding "owned" values
/// 
/// * This trait defines a type which is iterable of some type `T`
/// * This trait defines a type which can go to/from iterators with some predefined length using [`ExactSizeIterator`]
/// * This trait defines a type which can extend itself
/// 
/// Combining this with [`HasElement`] results in [`EncodedOutput`]
pub trait TranscodableIterable<T>: Extend<T> + AsRef<[T]> + FromIterator<T> + IntoIterator<Item = T> {}
impl<T, S> TranscodableIterable<T> for S
where
    S: Extend<T> + AsRef<[T]>,
    S: FromIterator<T> + IntoIterator<Item = T>,
    S::IntoIter: ExactSizeIterator,
{}

/// A trait to represent a type which has some sub-element of type `T`
/// 
/// Combining this with [`TranscodableIterable`] results in [`EncodedOutput`]
pub trait HasElement {
    type Element;
}
macro_rules! has_element {
    ($ty:ty, $elem:ty) => {
        has_element!(w_mut; $ty, $elem);
        has_element!(wo_mut; $ty, $elem);
    };
    ($ty:ty, $elem:ty; $($tt:tt)*) => {
        has_element!(w_mut; $ty, $elem; $($tt)*);
        has_element!(wo_mut; $ty, $elem; $($tt)*);
    };
    (wo_mut; $ty:ty, $elem:ty) => {
        impl HasElement for $ty {
            type Element = $elem;
        }
    };
    (wo_mut; $ty:ty, $elem:ty; $($tt:tt)*) => {
        impl<$($tt)*> HasElement for $ty {
            type Element = $elem;
        }
    };
    (w_mut; $ty:ty, $elem:ty) => {
        impl HasElement for &mut $ty {
            type Element = $elem;
        }
    };
    (w_mut; $ty:ty, $elem:ty; $($tt:tt)*) => {
        impl<$($tt)*> HasElement for &mut $ty {
            type Element = $elem;
        }
    };
}
has_element!(Vec<T>, T; T);
has_element!(Box<[T]>, T; T);
has_element!(wo_mut; &[T], T; T);
has_element!(wo_mut; &mut [T], T; T);
has_element!(String, char);
has_element!(wo_mut; &str, char);
has_element!(wo_mut; &mut str, char);
has_element!(dyn Iterator<Item = T>, T; T);