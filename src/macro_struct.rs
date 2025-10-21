use crate::{MacroArgs, generate_hardcoded_generics};
use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemStruct;

pub fn macro_struct(args: MacroArgs, input: ItemStruct) -> TokenStream {
    let MacroArgs {
        visibility,
        py_class_name,
        state_mappings,
    } = args;

    let input_clone = input.clone();
    let ident = input.ident;

    let generics = generate_hardcoded_generics(&state_mappings, &input.generics);

    let token = quote! {
        #input_clone

        #[pyo3::pyclass]
        #visibility struct #py_class_name {
            inner: #ident #generics
        }

        impl From< #ident #generics > for #py_class_name {
            fn from(item: #ident #generics) -> Self {
                Self {
                    inner: item
                }
            }
        }
    };

    token
}
