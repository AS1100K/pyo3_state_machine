use std::collections::HashMap;

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    GenericParam, Generics, Ident, Item, MetaNameValue, Token, parse::Parse, parse_macro_input,
    punctuated::Punctuated,
};

mod macro_enum;
mod macro_fn;
mod macro_impl;
mod macro_struct;

pub(crate) type StateMapping = HashMap<String, proc_macro2::TokenStream>;

pub(crate) struct MacroArgs {
    pub(crate) py_class_name: Ident,
    pub(crate) state_mappings: StateMapping,
}

impl Parse for MacroArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // 1. Parse the first part, which is expected to be an Identifier
        let py_class_name: Ident = input.parse()?;

        if !input.peek(Token![,]) {
            return Ok(Self {
                py_class_name,
                state_mappings: StateMapping::new(),
            });
        }

        // Consume the comma
        let _: Token![,] = input.parse()?;

        // 2. Parse all the state mappings
        let state_mappings: Punctuated<MetaNameValue, Token![,]> =
            Punctuated::parse_terminated(input)?;

        let mapping: StateMapping = state_mappings
            .iter()
            .map(|meta| {
                let key = meta.path.get_ident().unwrap().to_string();
                let value = meta.value.clone();

                let token = quote! { #value };

                (key, token)
            })
            .collect();

        Ok(Self {
            py_class_name,
            state_mappings: mapping,
        })
    }
}

#[proc_macro_attribute]
pub fn py_state_machine(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as MacroArgs);
    let input = parse_macro_input!(input as Item);

    match input {
        Item::Struct(item_struct) => macro_struct::macro_struct(args, item_struct).into(),
        Item::Enum(item_enum) => macro_enum::macro_enum(args, item_enum).into(),
        Item::Impl(item_impl) => macro_impl::macro_impl(args, item_impl).into(),
        Item::Fn(item_fn) => macro_fn::macro_fn(args, item_fn).into(),
        _ => syn::Error::new_spanned(
            &input,
            "py_state_machine macro is only available on struct, enum, fn, impl.",
        )
        .to_compile_error()
        .into(),
    }
}

pub(crate) fn generate_hardcoded_generics(
    state_mappings: &StateMapping,
    generics: &Generics,
) -> proc_macro2::TokenStream {
    let generic_args = generics.params.iter().map(|param| match param {
        GenericParam::Type(ty) => state_mappings
            .get(&ty.ident.to_string())
            .cloned()
            .unwrap_or_else(|| quote! { Please }),
        GenericParam::Const(c) => quote! { #c.ident },
        GenericParam::Lifetime(_) => {
            syn::Error::new_spanned(param, "Lifetimes are not allowed").into_compile_error()
        }
    });

    quote! { < #( #generic_args ),* > }
}
