// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;
use syn::parse::Parser;

#[macro_export]
/// Macro for handling values within a [`syn::MetaList::parse_nested_meta`] iterator. See the example at the bottom
/// for a working verbose use-case.
/// 
/// # Syntax
/// 
/// ```rust ignore
/// use tk_syn_macros::handle_unique_nested_meta_values;
/// 
/// // all input types to be optional
/// let foo: Option<syn::LitStr> = None;
/// let bar: Option<syn::Macro> = None;
/// let baz: Option<syn::LitBool> = None;
/// let qux: Option<syn::Type> = None;
/// let bep: Option<syn::Path> = None;
/// 
/// input.parse_nested_meta(|meta| {
///     handle_unique_nested_meta_values! {
///         meta;               // <-- syn::meta::ParseNestedMeta
///         "unknown field";    // <-- unknown field error message: &str
///         5;                  // <-- number of names to try to parse
/// 
///         foo: foo_parser,    // <-- parser fn: `fn(&syn::meta::ParseNestedMeta) -> Option<syn::Result<T>>`
///         bar: bar_parser,    //     with default error message if duplicate: "duplicate `bar` field"
///         
///         // using custom error message
///         baz: baz_parser => "there can only be one `baz` field because i said so!",
/// 
///         // using quick parser from `tk_syn_macros`
///         qux: tk_syn_macros::parse_pnm("qux"),
/// 
///         // using parser made using `tk_syn_macros::create_parser!`
///         bep: parser_maybestr_bep,
///     }
/// });
/// 
/// fn foo_parser(meta: &syn::meta::ParseNestedMeta) -> Option<syn::Result<syn::LitStr>>;
/// fn bar_parser(meta: &syn::meta::ParseNestedMeta) -> Option<syn::Result<syn::Macro>>;
/// fn baz_parser(meta: &syn::meta::ParseNestedMeta) -> Option<syn::Result<syn::LitBool>>;
/// 
/// // creates a name/value parser fn for keyword `bep` and value type `syn::Path`
/// // using value parser `parse_maybestr`
/// //
/// // `bep = <path>`       <-- valid, will return Some(Ok(<path>))
/// // `bep = <literal>`    <-- invalid, will return Some(Err)
/// // `baz = <path>`       <-- invalid, will return None
/// // 
/// // creates: `pub(crate) fn parser_maybestr_bep`
/// tk_syn_macros::create_parser! {
///     "bep": syn::Path;
///     parse_maybestr => syn::meta::ParseNestedMeta
/// }
/// 
/// // a function to parse a value which could be raw tokens, or surrounded by quotes
/// fn parse_maybestr<T: syn::parse::Parse>(input: &syn::meta::ParseNestedMeta) -> syn::Result<T> {
///     match input.value() {
///         Ok(value) => match value.parse::<syn::LitStr>() {
///             Ok(litstr) => litstr.parse(),
///             Err(_) => value.parse(),
///         },
///         Err(err) => Err(err),
///     }
/// }
/// ```
/// 
/// Each comma delimited tokenstream must the following crieteria:
/// 
/// * A unique keyword is expected, no duplicates
///   * For name-value pairs (e.g. `<name> = <value>,` ), the name is the keyword and the value should be returned by the `parse_fn`
///   * For paths (e.g. `<name>,` ), the name is the keyword and whether it exists or not should be returned by the `parse_fn`
///   * For lists (e.g. `<name>(<contents>),` ) the name is the keyword and the contents should be parsed by the `parse_fn`
/// 
/// Each `parser_fn` is defined by the [`tk_syn_macros::create_parser!`](crate::create_parser) macro or by the
/// [`tk_syn_macros::quick_parse`](crate::parse_pnm) function, where it returns:
/// 
/// * [`None`] if the keyword is not detected
/// * [`Some`] if the keyword is detected
///   * [`Ok`] if the value is successfully parsed
///   * [`Err`] if the value could not be parsed
/// 
/// Resulting in a return-type of [`Option<Result<T, syn::Error>>`](syn::Error). The [`syn::Error`] return type is required since
/// this macro is only expected to be used inside of [`syn::MetaList::parse_nested_meta`]
/// 
/// Each variable which holds these values must be of type [`Option<T>`], where `T` is the type returned by the `parser_fn`. Each
/// variable which doesn't have a custom error message will **default to the name of the variable**. E.g. if you expect a keyword 
/// to be an invalid variable name (like kebab-case or a reserved word), then the error message should be customized since:
/// 
/// ```rust ignore
/// // keyword = `var-iable`
/// let var: Option<syn::LitStr> = None;
/// 
/// input.parse_nested_meta(|meta| {
///     tk_syn_macros::handle_unique_nested_meta_values! {
///         meta; "unknown field"; 1;
///         var: tk_syn_macros::quick_parse_pnm("var-iable") => "duplicate `var-iable` field",
///     }
/// });
/// ```
///
/// # Example
/// 
/// ```rust
/// 
/// use const_format::concatcp;
/// 
/// const EXPECTED: &str = concat!("expected `type` or `encoder` or `decoder` or `var`");
/// 
/// #[derive(Debug, PartialEq)]
/// struct Example {
///     /// The name of the struct
///     pub name: syn::Path,
///     /// Required type field
///     pub typ: syn::Type,
///     /// Optional path to encoder
///     pub enc: Option<syn::Path>,
///     /// Optional path to decoder
///     pub dec: Option<syn::Path>,
///     /// Optional, defaults to `false`
///     pub var: bool,
/// }
/// 
/// impl Example {
/// 
///     fn parse_example_from_metalist(input: &syn::MetaList) -> syn::Result<Self> {
///     
///         let mut typ: Option<syn::Type> = None;
///         let mut encoder: Option<syn::Path> = None;
///         let mut decoder: Option<syn::Path> = None;
///         let mut var: Option<syn::LitBool> = None;
///         
///         input.parse_nested_meta(|meta| {
///             tk_syn_macros::handle_unique_nested_meta_values! {
///                 meta;                                           // <-- syn::meta::ParseNestedMeta
///                 concatcp!("unknown field, ", EXPECTED);         // <-- unknown field error message: &str
///                 4;                                              // <-- number of names to try to parse
///                 // vvv built-in parser function with keyword `type`, using custom error message vvv
///                 typ: tk_syn_macros::parse_pnm("type") => "duplicate `type` field! you can only have one!",
///                 encoder: parse_encoder_meta,                    // <-- custom parser function to handle both `encoder = <path>` and `encoder = "<path>"`
///                 decoder: tk_syn_macros::parse_pnm("decoder"),   // <-- built-in parser function with keyword `decoder`
///                 var: tk_syn_macros::parse_pnm("var"),           // <-- built-in parser function with keyword `var`
///             }
///         })?;
/// 
///         Ok(Example {
///             name: input.path.clone(),
///             typ: typ.ok_or(syn::Error::new_spanned(input.clone(), "missing required `type` field"))?,
///             enc: encoder,
///             dec: decoder,
///             var: match var {
///                 Some(var) => var.value,
///                 None => false,
///             },
///         })
///     
///     }
/// 
/// }
/// 
/// /// A custom parser to handle both:
/// /// * `encoder = <path>`
/// /// * `encoder = "<path>"`
/// fn parse_encoder_meta(meta: &syn::meta::ParseNestedMeta) -> Option<Result<syn::Path, syn::Error>> {
///     if !meta.path.is_ident("encoder") { return None; }
///     Some(match meta.value() {
///         Ok(value) => match value.parse::<syn::LitStr>() {
///             Ok(litstr) => litstr.parse(),
///             Err(_) => value.parse(),
///         },
///         Err(err) => Err(err),
///     })
/// }
/// 
/// fn main() {
///     // --------------------------------------------------
///     // parse correctly 1
///     // --------------------------------------------------
///     let input: syn::MetaList = syn::parse_str("
///         ident(
///             encoder = \"std::path::PathBuf\",
///             decoder = std::path::PathBuf,
///             type = u8,
///         )"
///     ).unwrap();
///     let output = Example::parse_example_from_metalist(&input);
///     assert!(output.is_ok());
///     assert_eq!(
///         output.unwrap(),
///         Example {
///             name: syn::parse_str("ident").unwrap(),
///             typ: syn::parse_str("u8").unwrap(),
///             enc: Some(syn::parse_str("std::path::PathBuf").unwrap()),
///             dec: Some(syn::parse_str("std::path::PathBuf").unwrap()),
///             var: false,
///         },
///     );
/// 
///     // --------------------------------------------------
///     // parse correctly 2
///     // --------------------------------------------------
///     let input: syn::MetaList = syn::parse_str("
///         foo(
///             type = f64,
///             encoder = std::path::PathBuf,
///             var = true,
///         )"
///     ).unwrap();
///     let output = Example::parse_example_from_metalist(&input);
///     assert!(output.is_ok());
///     assert_eq!(
///         output.unwrap(),
///         Example {
///             name: syn::parse_str("foo").unwrap(),
///             typ: syn::parse_str("f64").unwrap(),
///             enc: Some(syn::parse_str("std::path::PathBuf").unwrap()),
///             dec: None,
///             var: true,
///         },
///     );
/// 
///     // --------------------------------------------------
///     // unknown field
///     // --------------------------------------------------
///     let input: syn::MetaList = syn::parse_str("
///         bar(
///             weird_field = true,
///             encoder = std::path::PathBuf,
///             decoder = std::path::PathBuf,
///         )"
///     ).unwrap();
///     let output = Example::parse_example_from_metalist(&input);
///     assert!(output.is_err());
///     assert_eq!(
///         "unknown field, expected `type` or `encoder` or `decoder` or `var`",
///         output.unwrap_err().to_string(),
///     );
/// 
///     // --------------------------------------------------
///     // missing required field
///     // --------------------------------------------------
///     let input: syn::MetaList = syn::parse_str("
///         baz(
///             encoder = std::path::PathBuf,
///             decoder = std::path::PathBuf,
///         )"
///     ).unwrap();
///     let output = Example::parse_example_from_metalist(&input);
///     assert!(output.is_err());    
///     assert_eq!(
///         "missing required `type` field",
///         output.unwrap_err().to_string(),
///     );
/// 
///     // --------------------------------------------------
///     // duplicate field: default error message
///     // --------------------------------------------------
///     let input: syn::MetaList = syn::parse_str("
///         qux(
///             type = u32,
///             encoder = std::path::PathBuf,
///             decoder = std::path::PathBuf,
///             decoder = std::path::Path,
///         )"
///     ).unwrap();
///     let output = Example::parse_example_from_metalist(&input);
///     assert!(output.is_err());    
///     assert_eq!(
///         "duplicate `decoder` field",
///         output.unwrap_err().to_string(),
///     );
/// 
///     // --------------------------------------------------
///     // duplicate field: custom error message
///     // --------------------------------------------------
///     let input: syn::MetaList = syn::parse_str("
///         keyword(
///             type = f64,
///             type = u32,
///             encoder = std::path::PathBuf,
///             decoder = std::path::PathBuf,
///         )"
///     ).unwrap();
///     let output = Example::parse_example_from_metalist(&input);
///     assert!(output.is_err());    
///     assert_eq!(
///         "duplicate `type` field! you can only have one!",
///         output.unwrap_err().to_string(),
///     );
/// 
///     // --------------------------------------------------
///     // using non `<name> = <value>` syntax
///     // --------------------------------------------------
///     let input: syn::MetaList = syn::parse_str("
///         path(
///             type: f64,
///             encoder: std::path::PathBuf,
///         )"
///     ).unwrap();
///     let output = Example::parse_example_from_metalist(&input);
///     assert!(output.is_err());
///     assert_eq!(
///         "expected `=`",
///         output.unwrap_err().to_string(),
///     );
/// }
/// ```
macro_rules! handle_unique_nested_meta_values {
    (
        $meta:ident;
        $expected_msg:expr;
        $count:expr;
        $( $key:ident : $parse_fn:expr $(=> $error_msg:expr)? ),* $(,)?
    ) => {
        let mut __undetected = 0;

        $(
            $crate::handle_unique_nested_meta_values!(@parse_field $meta, $key, $parse_fn $(, $error_msg)?, __undetected);
        )*

        // --------------------------------------------------
        // if all keywords were undetected, then an unknown
        // keyword was found
        // --------------------------------------------------
        if __undetected == $count {
            return Err($meta.error($expected_msg));
        }

        Ok(())
    };
    
    // --------------------------------------------------
    // fields with custom error message
    // --------------------------------------------------
    (@parse_field $meta:ident, $key:ident, $parse_fn:expr, $error_msg:expr, $__undetected:expr) => {
        match $parse_fn(&$meta) {
            Some(Ok(x)) => match $key {
                Some(_) => return Err($meta.error($error_msg)),
                None => $key = Some(x),
            },
            Some(Err(err)) => return Err(err),
            None => $__undetected += 1,
        }
    };

    // --------------------------------------------------
    // fields with default error message
    // --------------------------------------------------
    (@parse_field $meta:ident, $key:ident, $parse_fn:expr, $__undetected:expr) => {
        match $parse_fn(&$meta) {
            Some(Ok(x)) => match $key {
                Some(_) => return Err($meta.error(concat!("duplicate `", stringify!($key), "` field"))),
                None => $key = Some(x),
            },
            Some(Err(err)) => return Err(err),
            None => $__undetected += 1,
        }
    };
}

#[macro_export]
/// Something
macro_rules! create_parser {
    // shorthand for parsing MetaNameValue using `crate::parse_nv`
    ($keyword:tt : $ty:ty ; nv) => {
        $crate::create_parser!(@emit $keyword : $ty; (crate) parse_nv => syn::MetaNameValue);
    };

    // shorthand for parsing ParseNestedMeta using `crate::parse_pnm`
    ($keyword:tt : $ty:ty ; pnm) => {
        $crate::create_parser!(@emit $keyword : $ty; (crate) parse_pnm => syn::meta::ParseNestedMeta);
    };

    // main parser implementation
    ($keyword:tt : $ty:ty ; $pname:path => $input_ty:ty  $(, $($args:expr),*)? ) => {
        $crate::create_parser!(@emit $keyword : $ty; (local) $pname => $input_ty  $(, $($args),*)?);
    };

    // final
    (@emit $keyword:tt : $ty:ty; ($where:ident) $pname:path => $input_ty:ty  $(, $($args:expr),*)?) => {
        ::paste::paste! {
            #[doc = concat!(
                " Try to parse a [`", stringify!($ty),
                "`] from a [`", stringify!($from_type_deref),
                "`] to assign to [`", stringify!($keyword), "`]."
            )]
            #[doc = ""]
            #[doc = " # Returns"]
            #[doc = ""]
            #[doc = concat!(" * [`None`] if the keyword [`", stringify!($keyword), "`] is not detected")]
            #[doc = concat!(" * [`Some`] if the keyword [`", stringify!($keyword), "`] is detected")]
            #[doc = "   * [`Ok`] if the value is parsed correctly"]
            #[doc = "   * [`Err`] if the value is not parsed correctly"]
            pub(crate) fn [<$pname:lower _ $keyword:lower>](input: &$input_ty)
                -> Option<::syn::Result<$ty>>
            {
                if $crate::create_parser!(@getif input $keyword) {
                    return None;
                }
                Some($crate::create_parser!(@getfn ($where) $pname)(input $(, $($args),*)?))
            }
        }
    };

    (@getif $input:ident $keyword:literal) => {
        !$input.path.is_ident($keyword)
    };

    (@getif $input:ident $keyword:ident) => {
        $input.path != $keyword
    };

    (@getfn (crate) $pname:ident) => {
        $crate::helpers::$pname
    };

    (@getfn (local) $pname:path) => {
        $pname
    };
}

/// A quick parser of [`syn::meta::ParseNestedMeta`], where the [`syn::meta::ParseNestedMeta::path`] is checked
/// against a [`str`] and it's [`syn::meta::ParseNestedMeta::value`] is parsed as-is into type `T`.
pub fn parse_pnm<T: syn::parse::Parse>(
    name: &str,
) -> impl Fn(&syn::meta::ParseNestedMeta) -> Option<Result<T, syn::Error>> + use<'_, T> {
    move |pnm| {
        match pnm.path.is_ident(name) {
            true => Some(match pnm.value() {
                Ok(value) => value.parse(),
                Err(err) => Err(err),
            }),
            false => None,
        }
    }
}

/// A quick parser of [`syn::MetaNameValue`], where the [`syn::MetaNameValue::path`] is checked
/// against a [`str`] and it's [`syn::MetaNameValue::value`] is parsed as-is into type `T`.
pub fn parse_nv<T: syn::parse::Parse>(
    name: &str,
) -> impl Fn(&syn::MetaNameValue) -> Option<Result<T, syn::Error>> + use<'_, T> {
    move |nv| {
        match nv.path.is_ident(name) {
            true => Some(T::parse.parse2(nv.value.to_token_stream())),
            false => None,
        }
    }
}

/// Helper functions for macro calls
pub mod helpers {
    use super::*;

    /// A parser of [`syn::meta::ParseNestedMeta`], where the [`syn::meta::ParseNestedMeta::value`] is parsed as-is into type `T`.
    pub fn parse_pnm<T>(pnm: &syn::meta::ParseNestedMeta) -> syn::Result<T>
    where
        T: syn::parse::Parse,
    {
        match pnm.value() {
            Ok(value) => value.parse(),
            Err(err) => Err(err),
        }
    }

    /// A parser of [`syn::MetaNameValue`], where the [`syn::MetaNameValue::value`] is parsed as-is into type `T`.
    pub fn parse_nv<T>(nv: &syn::MetaNameValue) -> syn::Result<T>
    where
        T: syn::parse::Parse,
    {
        T::parse.parse2(nv.value.to_token_stream())
    }
}