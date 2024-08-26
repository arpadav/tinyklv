# `symple`: `syn` made simple for proc-macro attribute parsing

Ever have trouble with `syn::Attribute::parse_args`, `syn::Attribute::parse_nested_meta`, etc.? Is this a skill issue? Definitely. But `symple` is simply a wrapper for `syn` to streamline processing of attributes.

***Authors note: This is by no means a replacement for `syn`, nor is this planned to be anything "production" quality in any way. `symple`'s sole purpose is to get proc-macro attribute parsing from 0 -> 90%, with the more granular details required to being handled by `syn`. In addition, a lot of what `symple` accomplishes can be done with modern `syn` features.***

## Idea

`symple` types can be broken down into the following types:

### Primitives using `symple::value::Value`

Can be of type:

* `syn::Lit` - a literal like `1` or `"foo"` or `true` or `0x02`
* `syn::Ident` - an identifier like `foo`
* `syn::Path` - a path like `foo::bar`
* `syn::Type` - a type like `foo::bar::<u64>`
* `syn::Expr` - an expression like `foo()` or `foo.bar()` or `|x| x + 1`

### Composites using `symple::item::MetaItem`

* `symple::tuple::Tuple` - a tuple like `foo(1, 2, 3)` or `foo(a = 1, b(x = 2), c = 3)`
  * A `Tuple` is made up of `<key>(<contents>)`
* `symple::value::Value` - like the examples above
  * A `Value` is made up of a stand-alone `<value>`
* `symple::nv::NameValue` - a name-value like `foo = 1` or `x = foo`
  * A `NameValue` is made up of `<name> = <value>`

### Recipe using `symple::contents::Contents`

* `symple::contents::Contents` - a contents like `a = 1, b(x = 2), c = 3, value`
  * A `Contents` is made up of `Vec<MetaItem>`, comma delimited

Right now, this is being exclusively used under the [tinyklv](https://crates.io/crates/tinyklv) crate
for parsing of it's proc-macro's. If enough development goes into this, might publish it as stand-alone crate as it could help with other of the authors projects.

## Usage

```rust ignore
// outside proc-macro lib
#[derive(MyProcMacro)]
#[my_proc_macro(value1 = 1, value2 = 2)]
// ^^ This is a `symple::Tuple` ^^
// key: my_proc_macro
// contents: { nv: { name = value1, value = 1 }, nv: { name = value2, value = 2 } }
struct SomeStruct {
    #[my_proc_macro(attr = "foo")]
    // ^^ This is a `symple::Tuple` ^^
    // key: my_proc_macro
    // contents: { nv: { name = attr, value = "foo" } }
    name: String,
    #[my_proc_macro(attr = "bar")]
    // ^^ This is a `symple::Tuple` ^^
    // key: my_proc_macro
    // contents: { nv: { name = attr, value = "bar" } }
    age: u32,
}
```

```rust ignore
// inside proc-macro lib

// struct attributes
struct StructAttributes {
    values: Tuple<Values>,
}
struct Values {
    value1: u32,
    value2: u32,
}

// field attributes
struct FieldAttribute {
    attr: symple::NameValue<syn::Lit>
}

// required for all items inside `symple::Tuple`
impl From<symple::MetaContents> for StructAttributes {
    fn from(x: symple::MetaContents) -> Self {
        todo!()
    }
}

// required for all items inside `symple::Tuple`
impl From<symple::MetaItem> for FieldAttributes {
    fn from(x: symple::MetaContents) -> Self {
        todo!()
    }
}

// required for all items inside `symple::NameValue`
impl From<symple::MetaValue> for FieldAttributes {
    fn from(x: symple::MetaValue) -> Self {
        self.attr = x.into()
    }
}
```

## Example implementations

* TODO: add tinyklv implementations here