/// Trait for encoding to bytes
pub trait Encode<T> {
    fn encode(&self) -> T;
}

/// Trait for decoding from bytes with a stream of input data
pub trait StreamDecode<I>: Sized
where
    I: winnow::stream::Stream,
{
    fn decode(input: &mut I) -> winnow::PResult<Self>;
}

/// Trait for parsing a value with variable (dynamic) length, decoded
/// during the parsing process
/// 
/// This is implemented for all functions with a signature of
/// * [Fn(&mut I, usize) -> winnow::PResult<O>](https://doc.rust-lang.org/std/ops/trait.Fn.html)
/// 
/// This is used during KLV parsing of the length when the `dyn` keyword is used
/// within the `klv` attribute for each field of the struct
/// 
/// # Example
/// 
/// ```rust ignore
/// ```
pub trait DynValParser<I, O>
where
    I: winnow::stream::Stream,
{
    fn dyn_val_parse(&mut self, input: &mut I, len: usize) -> winnow::PResult<O>;
}
impl<I, O, F> DynValParser<I, O> for F
where 
    I: winnow::stream::Stream,
    F: Fn(&mut I, usize) -> winnow::PResult<O>,
{
    fn dyn_val_parse(&mut self, input: &mut I, len: usize) -> winnow::PResult<O> {
        self(input, len)
    }
}

/// Trait for parsing a value with fixed length, decoded
/// during the parsing process
/// 
/// This is implemented for all functions with a signature of
/// * [Fn(&mut I) -> winnow::PResult<O>](https://doc.rust-lang.org/std/ops/trait.Fn.html)
/// 
/// This is used during KLV parsing of the length when ***no*** `dyn` keyword is used
/// within the `klv` attribute for each field of the struct
/// 
/// # Example
/// 
/// ```rust ignore
/// ```
pub trait FixValParser<I, O>
where
    I: winnow::stream::Stream,
{
    fn val_parse(&mut self, input: &mut I) -> winnow::PResult<O>;
}
impl<I, O, F> FixValParser<I, O> for F
where 
    I: winnow::stream::Stream,
    F: Fn(&mut I) -> winnow::PResult<O>,
{
    fn val_parse(&mut self, input: &mut I) -> winnow::PResult<O> {
        self(input)
    }
}