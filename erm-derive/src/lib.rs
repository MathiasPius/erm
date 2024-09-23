mod archetype;
mod component;
mod field;
//mod reflect;

use archetype::Archetype;
use component::Component;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, TokenStreamExt};
use syn::spanned::Spanned;

#[proc_macro_derive(Component, attributes(erm))]
pub fn derive_component(stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let stream = TokenStream::from(stream);
    let span = stream.span();
    let component: Component = syn::parse2(stream).unwrap();

    let implementation = |database: Ident, placeholder_char: char| {
        let sqlx = quote! {::sqlx};
        let database = quote! {#sqlx::#database};

        component.implementation(&sqlx, &database, placeholder_char)
    };

    implement_for(span, implementation).into()
}

#[proc_macro_derive(Archetype)]
pub fn derive_archetype(stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let stream = TokenStream::from(stream);
    let span = stream.span();
    let archetype: Archetype = syn::parse2(stream).unwrap();

    let implementation = |database: Ident, _: char| {
        let sqlx = quote! {::sqlx};
        let database = quote! {#sqlx::#database};

        archetype.implementation(&sqlx, &database)
    };

    implement_for(span, implementation).into()
}

fn implement_for(span: Span, implementer: impl Fn(Ident, char) -> TokenStream) -> TokenStream {
    let mut implementations = TokenStream::new();
    #[cfg(feature = "sqlite")]
    implementations.append_all(implementer(Ident::new("Sqlite", span), '?'));

    #[cfg(feature = "postgres")]
    implementations.append_all(implementer(Ident::new("Postgres", span), '$'));

    #[cfg(feature = "mysql")]
    implementations.append_all(implementer(Ident::new("MySql", span), '?'));

    implementations
}
