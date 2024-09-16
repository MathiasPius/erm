use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::Fields;

pub fn reflect_component(component_name: &Ident, fields: &Fields) -> TokenStream {
    let reflection_name = Ident::new(&format!("Reflected{component_name}"), component_name.span());

    let declarations = fields.iter().map(|field| {
        let name = field.ident.as_ref().unwrap();
        let typename = &field.ty;

        quote! {
            pub #name: ::erm::reflect::ReflectedColumn<#typename>
        }
    });

    let constructors = fields.iter().map(|field| {
        let name = field.ident.as_ref().unwrap();
        let stringified = name.to_string();

        quote! {
            #name: ::erm::reflect::ReflectedColumn::new(#stringified)
        }
    });

    quote! {
        pub struct #reflection_name {
            #(#declarations),*
        }

        impl #reflection_name {
            pub const fn new() -> Self {
                Self {
                    #(#constructors,)*
                }
            }
        }

        impl ::erm::reflect::Reflect for #component_name {
            type ReflectionType = #reflection_name;
            const FIELDS: Self::ReflectionType = #reflection_name::new();
        }
    }
}
