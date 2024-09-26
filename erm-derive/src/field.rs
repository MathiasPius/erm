use proc_macro2::{Ident, Literal, Punct, TokenStream};
use quote::quote;
use syn::spanned::Spanned;
use syn::Token;
use syn::{parse::Parse, Type};

pub enum Field {
    Numbered {
        ident: TokenStream,
        typename: Type,
        intermediate_type: Option<Type>,
        column_name: String,
    },
    Named {
        ident: TokenStream,
        typename: Type,
        intermediate_type: Option<Type>,
        column_name: String,
    },
}

impl Field {
    pub fn ident(&self) -> &TokenStream {
        match self {
            Field::Numbered { ident, .. } | Field::Named { ident, .. } => &ident,
        }
    }

    pub fn column_name(&self) -> &str {
        match self {
            Field::Numbered { column_name, .. } | Field::Named { column_name, .. } => &column_name,
        }
    }

    pub fn typename(&self) -> &Type {
        match self {
            Field::Numbered { typename, .. } | Field::Named { typename, .. } => &typename,
        }
    }

    pub fn intermediate(&self) -> Option<&Type> {
        match self {
            Field::Numbered {
                intermediate_type, ..
            }
            | Field::Named {
                intermediate_type, ..
            } => intermediate_type.as_ref(),
        }
    }

    pub fn column_definition(&self, sqlx: &TokenStream, database: &TokenStream) -> TokenStream {
        let name = self.column_name();
        let typename = self.typename();

        if let Some(intermediate) = self.intermediate() {
            quote! {
                ::erm::component::ColumnDefinition::<#database> {
                    name: #name,
                    type_info: <#intermediate as #sqlx::Type<#database>>::type_info(),
                }
            }
        } else {
            quote! {
                ::erm::component::ColumnDefinition::<#database> {
                    name: #name,
                    type_info: <#typename as #sqlx::Type<#database>>::type_info(),
                }
            }
        }
    }

    pub fn sql_definition(&self, sqlx: &TokenStream, database: &TokenStream) -> TokenStream {
        if let Some(intermediate) = self.intermediate() {
            quote! {
                <#intermediate as #sqlx::Type<#database>>::type_info().name(),
                if <#intermediate as #sqlx::Type<#database>>::type_info().is_null() {
                    "null"
                } else {
                    "not null"
                }
            }
        } else {
            let typename = self.typename();
            quote! {
                <#typename as #sqlx::Type<#database>>::type_info().name(),
                if <#typename as #sqlx::Type<#database>>::type_info().is_null() {
                    "null"
                } else {
                    "not null"
                }
            }
        }
    }

    pub fn serialize(&self) -> TokenStream {
        let name = self.ident();
        let typename = self.typename();

        if let Some(intermediate) = self.intermediate() {
            quote! {
                let query = query.bind(<&#typename as Into<#intermediate>>::into(&self.#name));
            }
        } else {
            quote! {
                let query = query.bind(&self.#name);
            }
        }
    }

    pub fn deserialize(&self) -> TokenStream {
        match self {
            Field::Numbered {
                ident,
                typename,
                intermediate_type,
                ..
            } => {
                let ident = Ident::new(&format!("self_{ident}"), ident.span());

                if let Some(intermediate) = &intermediate_type {
                    quote! {
                        let #ident: Result<#typename, _> = row.try_get::<#intermediate>().map(|field| <#typename as From<#intermediate>>::from(field));
                    }
                } else {
                    quote! {
                        let #ident = row.try_get::<#typename>();
                    }
                }
            }
            Field::Named {
                ident,
                typename,
                intermediate_type,
                ..
            } => {
                if let Some(intermediate) = &intermediate_type {
                    quote! {
                        let #ident: Result<#typename, _> = row.try_get::<#intermediate>().map(|field| <#typename as From<#intermediate>>::from(field));
                    }
                } else {
                    quote! {
                        let #ident = row.try_get::<#typename>();
                    }
                }
            }
        }
    }

    pub fn reflected_column(&self) -> TokenStream {
        let name = self.ident();
        let typename = self.typename();

        if let Some(intermediate) = self.intermediate() {
            quote! {
                pub #name: ::erm::reflect::ReflectedColumn<#intermediate>
            }
        } else {
            quote! {
                pub #name: ::erm::reflect::ReflectedColumn<#typename>
            }
        }
    }
}

impl TryFrom<(usize, syn::Field)> for Field {
    type Error = syn::Error;

    fn try_from((index, field): (usize, syn::Field)) -> Result<Self, Self::Error> {
        let attributes: Vec<_> = Result::<Vec<Vec<_>>, syn::Error>::from_iter(
            field
                .attrs
                .iter()
                .filter(|attr| attr.meta.path().is_ident("erm"))
                .map(|attr| {
                    let list = attr.meta.require_list()?;

                    Ok(syn::parse2::<FieldAttributeList>(list.tokens.clone())?.0)
                }),
        )?
        .into_iter()
        .flatten()
        .collect();

        let typename = field.ty.clone();

        let intermediate_type = attributes.iter().find_map(FieldAttribute::intermediate);

        let column_name = attributes
            .iter()
            .find_map(FieldAttribute::column)
            .or_else(|| field.ident.as_ref().map(ToString::to_string))
            .unwrap_or_else(|| format!("column{index}"));

        if let Some(ident) = field.ident {
            Ok(Field::Named {
                ident: quote! { #ident },
                typename,
                intermediate_type,
                column_name,
            })
        } else {
            let index = syn::Index::from(index);
            Ok(Field::Numbered {
                ident: quote! { #index },
                typename,
                intermediate_type,
                column_name,
            })
        }
    }
}

#[derive(Clone)]
pub struct FieldAttributeList(pub Vec<FieldAttribute>);

impl Parse for FieldAttributeList {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut attributes = Vec::new();

        while !input.is_empty() {
            attributes.push(FieldAttribute::parse(input)?);

            if input.peek(Token![,]) {
                input.parse::<Punct>()?;
            }
        }

        Ok(Self(attributes))
    }
}

#[derive(Clone)]
pub enum FieldAttribute {
    /// Changes the name of the field's column in the table
    Column { name: Literal },
    /// Intermediate type to convert to/from before storing in database.
    Intermediate { typename: Type },
}

impl FieldAttribute {
    pub fn column(&self) -> Option<String> {
        if let FieldAttribute::Column { name } = self {
            Some(name.to_string())
        } else {
            None
        }
    }

    pub fn intermediate(&self) -> Option<Type> {
        if let FieldAttribute::Intermediate { typename } = self {
            Some(typename.clone())
        } else {
            None
        }
    }
}

impl Parse for FieldAttribute {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;

        Ok(match ident.to_string().as_str() {
            "column" => {
                input.parse::<Token![=]>()?;

                FieldAttribute::Column {
                    name: input.parse()?,
                }
            }
            "intermediate" => {
                input.parse::<Token![=]>()?;

                FieldAttribute::Intermediate {
                    typename: input.parse()?,
                }
            }
            _ => return Err(syn::Error::new(ident.span(), "unexpected Field attribute")),
        })
    }
}
