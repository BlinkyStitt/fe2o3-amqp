use quote::quote;

use crate::{AmqpContractAttr, EncodingType, util::parse_described_attr};

pub(crate) fn expand_serialize(input: &syn::DeriveInput) -> Result<proc_macro2::TokenStream, syn::Error> {
    let attr = parse_described_attr(input);
    let ident = &input.ident;
    match &input.data {
        syn::Data::Struct(data) => {
            expand_serialize_on_struct(&attr, ident, data)
        },
        _ => unimplemented!()
    }
}

fn expand_serialize_on_struct(
    attr: &AmqpContractAttr, 
    ident: &syn::Ident, 
    data: &syn::DataStruct
) -> Result<proc_macro2::TokenStream, syn::Error> 
{
    let ident_name = ident.to_string();
    let descriptor = match attr.code {
        Some(code) => quote!(fe2o3_amqp::constants::DESCRIPTOR_CODE, #code),
        None => quote!(fe2o3_amqp::constants::DESCRIPTOR_NAME, #ident_name)
    };
    let name = match attr.encoding {
        EncodingType::Basic => quote!(fe2o3_amqp::constants::DESCRIBED_BASIC),
        EncodingType::List => quote!(fe2o3_amqp::constants::DESCRIBED_LIST),
        EncodingType::Map => quote!(fe2o3_amqp::constants::DESCRIBED_MAP)
    };
    let field_idents: Vec<syn::Ident> = data.fields.iter().map(|f| f.ident.clone().unwrap()).collect();
    let field_names: Vec<String> = field_idents.iter().map(|i| i.to_string()).collect();
    let len = field_idents.len();
    let token = quote! {
        impl serde::ser::Serialize for #ident {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::ser::Serializer,
            {
                use serde::ser::SerializeStruct;
                let mut state = serializer.serialize_struct(#name, #len + 1)?;
                state.serialize_field(#descriptor)?;
                #( state.serialize_field(#field_names, &self.#field_idents)?; )*
                state.end()
            }
        }
    };
    Ok(token)
}