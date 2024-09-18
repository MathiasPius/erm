use proc_macro2::Punct;
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    Attribute, Error, Ident, Meta,
};

#[derive(Debug)]
enum ComponentAttribute {
    StoreAs(Ident),
    SerializeAs(Ident),
    DeserializeAs(Ident),
}

impl Parse for ComponentAttribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        match input.parse::<Ident>()?.to_string().as_str() {
            "store_as" => {
                let punctuation = input.parse::<Punct>()?;

                if punctuation.as_char() != '=' {
                    return Err(Error::new(punctuation.span(), "unexpected punctuation"));
                }

                Ok(ComponentAttribute::StoreAs(input.parse::<Ident>()?))
            }
            "serialize_as" => {
                let punctuation = input.parse::<Punct>()?;

                if punctuation.as_char() != '=' {
                    return Err(Error::new(punctuation.span(), "unexpected punctuation"));
                }

                Ok(ComponentAttribute::SerializeAs(input.parse::<Ident>()?))
            }
            "deserialize_as" => {
                let punctuation = input.parse::<Punct>()?;

                if punctuation.as_char() != '=' {
                    return Err(Error::new(punctuation.span(), "unexpected punctuation"));
                }

                Ok(ComponentAttribute::DeserializeAs(input.parse::<Ident>()?))
            }
            other => Err(Error::new(
                other.span(),
                format!("unexpected attribute: {}", other),
            )),
        }
    }
}

#[derive(Debug, Default)]
pub struct ComponentAttributes {
    pub deser_as: Option<Ident>,
    pub ser_as: Option<Ident>,
    pub store_as: Option<Ident>,
}

impl ComponentAttributes {
    pub fn from_attributes(attributes: &Vec<Attribute>) -> syn::Result<Self> {
        let mut component_attributes = Self::default();

        for attribute in attributes.iter() {
            let Meta::List(list) = &attribute.meta else {
                continue;
            };

            if list.path.require_ident()? != "erm" {
                continue;
            }

            match syn::parse2(list.tokens.clone())? {
                ComponentAttribute::StoreAs(ident) => {
                    component_attributes.ser_as.get_or_insert(ident.clone());
                    component_attributes.deser_as.get_or_insert(ident.clone());
                    component_attributes.store_as = Some(ident);
                }
                ComponentAttribute::SerializeAs(ident) => {
                    component_attributes.ser_as = Some(ident.clone());
                    component_attributes.deser_as.get_or_insert(ident.clone());
                    component_attributes.store_as.get_or_insert(ident);
                }
                ComponentAttribute::DeserializeAs(ident) => {
                    component_attributes.ser_as.get_or_insert(ident.clone());
                    component_attributes.deser_as = Some(ident.clone());
                    component_attributes.store_as.get_or_insert(ident);
                }
            }
        }

        Ok(component_attributes)
    }
}
