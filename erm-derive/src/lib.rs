mod archetype;
mod component;
mod field;
mod reflect;

use archetype::Archetype;
use component::Component;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, TokenStreamExt};
use reflect::reflect_component;
use syn::spanned::Spanned;

#[proc_macro_derive(Component, attributes(erm))]
pub fn derive_component(stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let stream = TokenStream::from(stream);
    let span = stream.span();
    let component: Component = syn::parse2(stream).unwrap();

    let implementation = |database: Ident, placeholder_char: char| {
        #[cfg(feature = "bundled")]
        let sqlx = quote! {::erm::sqlx};
        #[cfg(not(feature = "bundled"))]
        let sqlx = quote! {::sqlx};

        let database = quote! {#sqlx::#database};

        component.implementation(&sqlx, &database, placeholder_char)
    };

    let mut implementations = implement_for(span, implementation);
    implementations.append_all(reflect_component(&component.typename, &component.fields));
    implementations.into()
}

#[proc_macro_derive(Archetype, attributes(erm))]
pub fn derive_archetype(stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let stream = TokenStream::from(stream);
    let span = stream.span();
    let archetype: Archetype = syn::parse2(stream).unwrap();

    let implementation = |database: Ident, _: char| {
        #[cfg(feature = "bundled")]
        let sqlx = quote! {::erm::sqlx};
        #[cfg(not(feature = "bundled"))]
        let sqlx = quote! {::sqlx};
        let database = quote! {#sqlx::#database};

        archetype.implementation(&sqlx, &database)
    };

    implement_for(span, implementation).into()
}

#[allow(unused)]
fn implement_for(span: Span, implementer: impl Fn(Ident, char) -> TokenStream) -> TokenStream {
    #[allow(unused_mut)]
    let mut implementations = TokenStream::new();

    #[cfg(feature = "sqlite")]
    implementations.append_all(implementer(Ident::new("Sqlite", span), '?'));

    #[cfg(feature = "postgres")]
    implementations.append_all(implementer(Ident::new("Postgres", span), '$'));

    #[cfg(feature = "mysql")]
    implementations.append_all(implementer(Ident::new("MySql", span), '?'));

    implementations
}
