use proc_macro::TokenStream;
use syn::{
    Ident, Item, MetaNameValue, Token, parse::Parse, parse_macro_input, punctuated::Punctuated,
};

mod macro_enum;
mod macro_fn;
mod macro_impl;
mod macro_struct;

pub(crate) struct MacroArgs {
    pub(crate) py_class_name: Ident,
    pub(crate) state_mappings: Punctuated<MetaNameValue, Token![,]>,
}

impl Parse for MacroArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // 1. Parse the first part, which is expected to be an Identifier
        let py_class_name: Ident = input.parse()?;

        if !input.peek(Token![,]) {
            return Ok(Self {
                py_class_name,
                state_mappings: Punctuated::new(),
            });
        }

        // Consume the comma
        let _: Token![,] = input.parse()?;

        // 2. Parse all the state mappings
        let state_mappings = Punctuated::parse_terminated(input)?;

        Ok(Self {
            py_class_name,
            state_mappings,
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
