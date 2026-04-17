//! Field-value coercion for `#[klv(enc = &func)]` dispatch.
//!
//! Author: aav
// --------------------------------------------------
// external
// --------------------------------------------------
use std::borrow::Cow;
use std::rc::Rc;
use std::sync::Arc;

/// Coerce a field value into the shape its encoder expects.
///
/// Used by `#[klv(enc = &func)]` to route field values through a zero-cost
/// borrowed form - `Copy` scalars pass by value, `String` becomes `&str`,
/// `Vec<T>` becomes `&[T]`, and smart-pointer wrappers resolve to inner refs.
/// No clones, no heap allocations.
///
/// # When to implement
///
/// You do **not** need to implement this for custom struct types. The no-sigil
/// form `#[klv(enc = func)]` passes `&self.field` directly to the encoder and
/// covers all custom-struct encoders like `MyType::encode_value(&self)`.
///
/// Implement `EncodeAs` only when you want your type to be usable with the
/// `&` sigil and dispatch to a distinct borrowed form (e.g. a newtype around
/// `String` that should coerce to `&str`).
///
/// # Example
///
/// ```rust
/// use tinyklv::prelude::*;
///
/// // encoder takes &str, not &String
/// fn encode_str(s: &str) -> Vec<u8> { s.as_bytes().to_vec() }
///
/// let s = String::from("hi");
/// let borrowed: &str = EncodeAs::encode_as(&s);
/// assert_eq!(encode_str(borrowed), b"hi".to_vec());
/// ```
pub trait EncodeAs {
    /// The borrowed form produced for encoder consumption.
    type Borrowed<'a>
    where
        Self: 'a;

    /// Produce the borrowed form.
    fn encode_as(&self) -> Self::Borrowed<'_>;
}

/// primitive scalars: by value (Copy, zero cost)
macro_rules! impl_encode_as_copy {
    ($($t:ty),* $(,)?) => { $(
        impl EncodeAs for $t {
            type Borrowed<'a> = $t;
            #[inline(always)]
            fn encode_as(&self) -> $t { *self }
        }
    )* };
}
impl_encode_as_copy!(
    u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64, bool, char,
);

/// [`EncodeAs`] implementation for [`String`]
impl EncodeAs for String {
    type Borrowed<'a> = &'a str;

    #[inline(always)]
    fn encode_as(&self) -> &str {
        self.as_str()
    }
}

/// [`EncodeAs`] implementation for [`Vec`]
impl<T> EncodeAs for Vec<T> {
    type Borrowed<'a>
        = &'a [T]
    where
        T: 'a;

    #[inline(always)]
    fn encode_as(&self) -> &[T] {
        self.as_slice()
    }
}

/// [`EncodeAs`] implementation for [`Cow`] strings
impl<'b> EncodeAs for Cow<'b, str> {
    type Borrowed<'a>
        = &'a str
    where
        Self: 'a;

    #[inline(always)]
    fn encode_as(&self) -> &str {
        self.as_ref()
    }
}

/// [`EncodeAs`] implementation for [`Cow`] slices
impl<'b, T: Clone> EncodeAs for Cow<'b, [T]> {
    type Borrowed<'a>
        = &'a [T]
    where
        Self: 'a,
        T: 'a;

    #[inline(always)]
    fn encode_as(&self) -> &[T] {
        self.as_ref()
    }
}

/// [`EncodeAs`] implementation for [`Box`]
impl<T: ?Sized> EncodeAs for Box<T> {
    type Borrowed<'a>
        = &'a T
    where
        T: 'a;

    #[inline(always)]
    fn encode_as(&self) -> &T {
        self
    }
}

/// [`EncodeAs`] implementation for [`Rc`]
impl<T: ?Sized> EncodeAs for Rc<T> {
    type Borrowed<'a>
        = &'a T
    where
        T: 'a;

    #[inline(always)]
    fn encode_as(&self) -> &T {
        self
    }
}

/// [`EncodeAs`] implementation for [`Arc`]
impl<T: ?Sized> EncodeAs for Arc<T> {
    type Borrowed<'a>
        = &'a T
    where
        T: 'a;

    #[inline(always)]
    fn encode_as(&self) -> &T {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitives_are_copied_by_value() {
        let x: u32 = 42;
        let y: u32 = EncodeAs::encode_as(&x);
        assert_eq!(x, y);
        let b: bool = true;
        let bb: bool = EncodeAs::encode_as(&b);
        assert!(bb);
    }

    #[test]
    fn string_borrows_as_str() {
        let s = String::from("hello");
        let b: &str = EncodeAs::encode_as(&s);
        assert_eq!(b, "hello");
        assert_eq!(b.as_ptr(), s.as_ptr());
    }

    #[test]
    fn vec_borrows_as_slice() {
        let v = vec![1u8, 2, 3];
        let b: &[u8] = EncodeAs::encode_as(&v);
        assert_eq!(b, &[1, 2, 3]);
        assert_eq!(b.as_ptr(), v.as_ptr());
    }

    #[test]
    fn cow_str_borrows_as_str() {
        let c: Cow<'_, str> = Cow::Borrowed("hi");
        let b: &str = EncodeAs::encode_as(&c);
        assert_eq!(b, "hi");
    }

    #[test]
    fn cow_slice_borrows_as_slice() {
        let c: Cow<'_, [u8]> = Cow::Owned(vec![0u8, 1, 2]);
        let b: &[u8] = EncodeAs::encode_as(&c);
        assert_eq!(b, &[0, 1, 2]);
    }

    #[test]
    fn box_borrows_inner() {
        let b: Box<u32> = Box::new(7);
        let r: &u32 = EncodeAs::encode_as(&b);
        assert_eq!(*r, 7);
    }

    #[test]
    fn box_str_borrows_str() {
        let b: Box<str> = Box::from("boxed");
        let r: &str = EncodeAs::encode_as(&b);
        assert_eq!(r, "boxed");
    }

    #[test]
    fn rc_and_arc_borrow_inner() {
        let r: Rc<u32> = Rc::new(3);
        let a: Arc<u32> = Arc::new(9);
        let rr: &u32 = EncodeAs::encode_as(&r);
        let ar: &u32 = EncodeAs::encode_as(&a);
        assert_eq!(*rr, 3);
        assert_eq!(*ar, 9);
    }
}
