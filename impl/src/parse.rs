// --------------------------------------------------
// external
// --------------------------------------------------

pub(crate) fn unwrap_option_type(ty: &syn::Type) -> Option<&syn::Type> {
    is_option_helper(ty).1
}

pub(crate) fn is_option(ty: &syn::Type) -> bool {
    is_option_helper(ty).0
}

fn is_option_helper(ty: &syn::Type) -> (bool, Option<&syn::Type>) {
    if let syn::Type::Path(syn::TypePath {
        path,
        ..
    }) = ty {
        if let Some(syn::PathSegment {
            ident: ref id,
            arguments: syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                args,
                ..
            })
        }) = path.segments.first() {
            if id == "Option" {
                return (true, args.first().and_then(|arg| match arg {
                    syn::GenericArgument::Type(inner_ty) => Some(inner_ty),
                    _ => None,
                }))
            }
        }
    }
    (false, None)
}

pub(crate) fn u8_slice() -> syn::Type {
    syn::Type::Reference(syn::TypeReference {
        and_token: Default::default(),
        lifetime: None,
        mutability: None,
        elem: Box::new(syn::Type::Slice(syn::TypeSlice {
            bracket_token: Default::default(),
            elem: Box::new(syn::Type::Path(syn::TypePath {
                qself: None,
                path: syn::parse_quote! { u8 },
            })),
        })),
    })
}