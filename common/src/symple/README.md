# `symple`: `syn` made simple for proc-macro attribute parsing

Ever have trouble with `syn::Attribute::parse_args`, `syn::Attribute::parse_nested_meta`, `syn::parse_macro_input!(input as syn::AttributeArgs)` etc.? Do you get this error: `error: expected identifier or literal`? Is this a skill issue? Definitely.

But `symple` is simply a wrapper for `syn` to streamline processing of attributes without some of the constraints of proc-macro parsing imposed by `syn`.

***Authors note:***

* ***This is by no means a replacement for `syn`, nor is this planned to be anything "production" quality in any way.***
* ***`symple`'s sole purpose is to get proc-macro attribute parsing from 0 -> 80%, with the more granular details required to being handled by `syn`.***
* ***Since this is a beta release, please feel free to add additional implementations. I definitely do not expect to have all cases covered.***
* ***In addition, a lot of what `symple` accomplishes can be done with modern `syn` features, albeit with boilerplate.***

## Idea

`symple` types can be broken down into the following types:

### Recipe using `symple::Contents`

* `symple::Contents` - a contents like `a = 1, b(x = 2), c = 3, value`
  * A `Contents` is made up of `Vec<symple::MetaItem>`, comma delimited

### Composites using `symple::MetaItem`

* `symple::Tuple` - a tuple like `foo(1, 2, 3)` or `foo(a = 1, b(x = 2), c = 3)`
  * A `Tuple` is made up of `<key>(<contents>)`
* `symple::Value` - like the examples above
  * A `Value` is made up of a stand-alone `<value>`
* `symple::NameValue` - a name-value like `foo = 1` or `x = foo`
  * A `NameValue` is made up of `<name> = <value>`

### Primitives using `symple::Value`

* `syn::Lit` - a literal like `1` or `"foo"` or `true` or `0x02`
* `syn::Ident` - an identifier like `foo`
* `syn::Path` - a path like `foo::bar`
* `syn::Type` - a type like `foo::bar::<u64>`
* `syn::Expr` - an expression like `foo()` or `foo.bar()` or `|x| x + 1`

Right now, this has exclusively been developed for the [tinyklv](https://crates.io/crates/tinyklv) crate
for parsing of it's proc-macro's. If enough development goes into this, might publish it as stand-alone crate as it could help with other of the authors projects.

## Usage

```rust ignore
/// Struct attributes
/// 
/// See example below, and example in README.md
struct StructAttributes {
    values: symple::Tuple<Values>,
}
#[derive(Default)]
struct Values {
    value1: symple::NameValue<syn::LitInt>,
    value2: symple::NameValue<syn::LitInt>,
}

/// Field attributes
/// 
/// See example below, and example in README.md
#[derive(Default)]
struct FieldAttribute {
    attr: symple::NameValue<syn::Lit>
}

/// [Values] implementation of [From] for [symple::MetaContents]
/// 
/// This is required for all items inside [symple::Tuple]
/// 
/// This is an example parsing implementation using [symple] types
/// 
/// See example below, and example in README.md
impl From<symple::MetaContents> for Values {
    fn from(x: symple::MetaContents) -> Self {
        let mut output = Values::default();
        for item in x.into_iter() {
            match item {
                symple::MetaItem::Tuple(tpl) => {
                    if tpl.name.to_string() != "my_proc_macro" { continue; }
                    for item in tpl.into_iter() {
                        if let symple::MetaItem::NameValue(mnv) = item {
                            match mnv.name.to_string().as_str() {
                                "value1" => output.value1 = mnv.value.clone().into(),
                                "value2" => output.value2 = mnv.value.clone().into(),
                                _ => (),
                            }
                        }
                    }
                }
                _ => (),
            }
        }
        output
    }
}

/// [syn::LitInt] implementation of [From] for [symple::MetaValue]
/// 
/// This is required for all items inside [symple::NameValue]
/// 
/// This is an example parsing implementation using [symple] types
/// 
/// See example below, and example in README.md
impl From<symple::MetaValue> for syn::LitInt {
    fn from(x: symple::MetaValue) -> Self {
        match x {
            symple::MetaValue::Lit(syn::Lit::Int(lit_int)) => lit_int,
            _ => panic!("Failed to convert MetaValue to LitInt"),
        }
    }
}

/// [FieldAttribute] implementation of [From] for [symple::MetaValue]
/// 
/// This is required for all items inside [symple::NameValue]
/// 
/// This is an example parsing implementation using [symple] types
/// 
/// See example below, and example in README.md
impl From<symple::MetaValue> for FieldAttribute {
    fn from(x: symple::MetaValue) -> Self {
        let mut output = FieldAttribute::default();
        output.attr = x.into();
        output
    }
}

#[test]
/// Parses the struct attributes of the following:
/// 
/// ```ignore
/// #[derive(MyProcMacro)]
/// #[my_proc_macro(value1 = 1, value2 = 2)]
/// // ^^ This is a `symple::Tuple` ^^
/// // key: my_proc_macro
/// // contents: { nv: { name = value1, value = 1 }, nv: { name = value2, value = 2 } }
/// struct SomeStruct {
///     #[my_proc_macro(attr = "foo")]
///     // ^^ This is a `symple::Tuple` ^^
///     // key: my_proc_macro
///     // contents: { nv: { name = attr, value = "foo" } }
///     name: String,
///     #[my_proc_macro(attr = "bar")]
///     // ^^ This is a `symple::Tuple` ^^
///     // key: my_proc_macro
///     // contents: { nv: { name = attr, value = "bar" } }
///     age: u32,
/// }
/// ```
fn parse_struct_attributes() {
    let input = quote::quote! {
        my_proc_macro(value1 = 1, value2 = 2)
    };
    let meta = syn::parse2::<MetaContents>(input);
    assert!(meta.is_ok());
    let meta = meta.unwrap();
    let values = Values::from(meta);
    assert!(values.value1.value.is_some());
    assert!(values.value2.value.is_some());
    assert_eq!(values.value1.value.unwrap().base10_digits(), "1");
    assert_eq!(values.value2.value.unwrap().base10_digits(), "2");
}

#[test]
/// Parses the field attributes of the following:
/// 
/// ```ignore
/// #[derive(MyProcMacro)]
/// #[my_proc_macro(value1 = 1, value2 = 2)]
/// // ^^ This is a `symple::Tuple` ^^
/// // key: my_proc_macro
/// // contents: { nv: { name = value1, value = 1 }, nv: { name = value2, value = 2 } }
/// struct SomeStruct {
///     #[my_proc_macro(attr = "foo")]
///     // ^^ This is a `symple::Tuple` ^^
///     // key: my_proc_macro
///     // contents: { nv: { name = attr, value = "foo" } }
///     name: String,
///     #[my_proc_macro(attr = "bar")]
///     // ^^ This is a `symple::Tuple` ^^
///     // key: my_proc_macro
///     // contents: { nv: { name = attr, value = "bar" } }
///     age: u32,
/// }
/// ```
fn parse_field_attributes() {
    let input = quote::quote! {
        my_proc_macro(attr = "foo")
    };
    let meta = syn::parse2::<MetaTuple>(input);
    assert!(meta.is_ok());
    let meta = meta.unwrap();
    for item in meta.contents.into_iter() {
        if let MetaItem::NameValue(mnv) = item {
            assert_eq!(mnv.name.to_string(), "attr");
            assert_eq!(mnv.value.to_string(), "\"foo\"");
            let field_attribute = FieldAttribute::from(mnv.value.clone());
            assert!(true);
            assert_eq!(field_attribute.attr.to_string(), "\"foo\"");
            break;
        }
    }
}
```

## Example implementations

* In most datatype files in `symple`, there are examples in docs and tests
* TODO: add tinyklv implementations here
