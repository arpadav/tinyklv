pub fn u8_slice() -> syn::Type {
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