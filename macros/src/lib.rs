#[macro_export]
macro_rules! debug_from_display {
    ($t:ident, $($constraint:tt)*) => {
        impl<T> std::fmt::Debug for $t<T>
        where
            T: $($constraint)*
        {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self)
            }
        }
    };
    ($t:ty) => {
        impl std::fmt::Debug for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self)
            }
        }
    };
}
