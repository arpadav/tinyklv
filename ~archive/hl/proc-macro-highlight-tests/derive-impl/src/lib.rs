// --------------------------------------------------
// external
// --------------------------------------------------
use syn::{
    Token,
    DeriveInput,
    parse::Parse,
    parse_macro_input,
};
use quote::{
    quote,
    format_ident,
};

const ATTR_NAME: &str = "my_attr";
const ATTR_FIELD_NAME: &str = "other_fn";

#[proc_macro_derive(MyProcMacro, attributes(my_attr))]
pub fn my_macro_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    
    let struct_name = input.ident;
    let mut fn_name = None::<syn::Path>;

    let syn::Data::Struct(input_data) = input.data else {
        panic!("Expected struct");
    };

    for f in input_data.fields {
        for attr in f.attrs {
            if attr.path().is_ident(ATTR_NAME) {
                let _ = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident(ATTR_FIELD_NAME) {
                        if let Ok(syn::Lit::Str(lit_str)) = meta.value().and_then(syn::Lit::parse) {
                            fn_name = Some(lit_str.parse()?);
                            Ok(())
                        } else {
                            Err(meta.error(&format!("Expected string literal for '{}'", ATTR_FIELD_NAME)))
                        }
                    } else {
                        Err(meta.error("Unexpected meta item"))
                    }
                });
            }
        }
    }

    let fn_name = match fn_name {
        Some(name) => name,
        None => panic!("Missing '{0}' attribute in #[my_attr({0} = \"...\")].", ATTR_FIELD_NAME),
    };

    let attr_field_ident = format_ident!("{}", ATTR_FIELD_NAME);

    let output = quote! {
        impl MyTrait for #struct_name {
            fn #attr_field_ident (input: u8) -> u8 {
                #fn_name(input)
            }
        }
    };

    output.into()
}

const ATTR2_NAME: &str = "my_attr";
const ATTR2_FIELD_NAME: &str = "nested";
use syn::punctuated::Punctuated;

#[proc_macro_derive(MyProcMacroNested, attributes(my_attr))]
pub fn my_macro_derive_nested(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    
    let struct_name = input.ident;
    let mut one_fn_name = None::<syn::Path>;
    let mut two_fn_name = None::<syn::Path>;

    let mut fn_name = None::<syn::Path>;

    let syn::Data::Struct(input_data) = input.data else {
        panic!("Expected struct");
    };
    let mut idx = 0;

    for f in input_data.fields {
        for attr in f.attrs {
            if attr.path().is_ident(ATTR2_NAME) {
                
                // let _ = attr.parse_nested_meta(|meta| {
                //     if meta.path.is_ident(ATTR2_FIELD_NAME) {
                //         let nested: NestedAttr = meta.input.parse()?;
                //         one_fn_name = Some(nested.one_fn);
                //         two_fn_name = Some(nested.two_fn);
                //         Ok(())
                //     } else if meta.path.is_ident(ATTR_FIELD_NAME) {
                //         if let Ok(syn::Lit::Str(lit_str)) = meta.value().and_then(syn::Lit::parse) {
                //             fn_name = Some(lit_str.parse()?);
                //             Ok(())
                //         } else {
                //             Err(meta.error(&format!("Expected string literal for '{}'", ATTR_FIELD_NAME)))
                //         }
                //     } else {
                //         Err(meta.error("Unexpected meta item"))
                //     }
                // });

                let Ok(nested) = attr.parse_args_with(Punctuated::<syn::Meta, Token![,]>::parse_terminated) else { continue };
                for meta in nested {
                    idx += 1;
                    match meta {
                        syn::Meta::List(list) => {
                            if list.path.is_ident(ATTR2_FIELD_NAME) {
                                let _ = list.parse_nested_meta(|meta| {
                                    if meta.path.is_ident("one_fn") {
                                        one_fn_name = Some(meta.value()?.parse()?);
                                    } else if meta.path.is_ident("two_fn") {
                                        two_fn_name = Some(meta.value()?.parse()?);
                                    }
                                    Ok(())
                                });
                            }
                        }
                        syn::Meta::NameValue(name_value) => {
                            if let syn::Expr::Lit(expr) = name_value.value {
                                if let syn::Lit::Str(lit_str) = expr.lit {
                                    fn_name = lit_str.parse().ok();
                                }
                            }
                        }
                        syn::Meta::Path(path) => {} // for allow unimplemented flags
                    }
                };
            }
        }
    }

    // panic!("{idx}");

    let one_fn_name = match one_fn_name {
        Some(name) => name,
        None => panic!("Missing '{0}' attribute in #[my_attr({0}(one_fn = \"...\")].", ATTR2_FIELD_NAME, ),
    };

    let two_fn_name = match two_fn_name {
        Some(name) => name,
        None => panic!("Missing '{0}' attribute in #[my_attr({0}(two_fn = \"...\")].", ATTR2_FIELD_NAME, ),
    };

    let one_field_ident = format_ident!("one_fn");
    let two_field_ident = format_ident!("two_fn");

    let mut output = quote! {
        impl MyTraitNested for #struct_name {
            fn #one_field_ident (input: u8) -> u8 {
                #one_fn_name(input)
            }

            fn #two_field_ident (input: u8) -> u8 {
                #two_fn_name(input)
            }
        }
    };

    match fn_name {
        Some(name) => {
            let fn_field_ident = format_ident!("{}", ATTR_FIELD_NAME);
            output.extend(quote! {
                impl MyTrait for #struct_name {
                    fn #fn_field_ident (input: u8) -> u8 {
                        #name(input)
                    }
                }
            });
        }
        None => {
            panic!("Missing '{0}' attribute in #[my_attr({0} = \"...\")].", ATTR_FIELD_NAME);
        }
    }

    output.into()
}

struct NestedAttr {
    pub one_fn: syn::Path,
    pub two_fn: syn::Path,
}

impl syn::parse::Parse for NestedAttr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        syn::parenthesized!(content in input); // Handle the parenthesis

        let mut one_fn: Option<syn::Path> = None;
        let mut two_fn: Option<syn::Path> = None;

        while !content.is_empty() {
            let key: syn::Ident = content.parse()?;
            content.parse::<Token![=]>()?;
            // let value_lit: syn::LitStr = content.parse()?;
            // let value_path: syn::Path = value_lit.parse()?;

            let value_path: syn::Path = content.parse()?;

            match &*key.to_string() {
                "one_fn" => {
                    if one_fn.is_some() {
                        return Err(content.error("duplicate `one_fn` field"));
                    }
                    one_fn = Some(value_path);
                }
                "two_fn" => {
                    if two_fn.is_some() {
                        return Err(content.error("duplicate `two_fn` field"));
                    }
                    two_fn = Some(value_path);
                }
                _ => return Err(content.error("unexpected field")),
            }

            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }

        Ok(NestedAttr {
            one_fn: one_fn.ok_or_else(|| input.error("missing `one_fn` field"))?,
            two_fn: two_fn.ok_or_else(|| input.error("missing `two_fn` field"))?,
        })
    }
}
