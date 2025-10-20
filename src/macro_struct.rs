use crate::{MacroArgs, generate_hardcoded_generics};
use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemStruct;

pub fn macro_struct(args: MacroArgs, input: ItemStruct) -> TokenStream {
    let MacroArgs {
        py_class_name,
        state_mappings,
    } = args;

    let input_clone = input.clone();
    let ident = input.ident;

    let generics = generate_hardcoded_generics(&state_mappings, &input.generics);

    let token = quote! {
        #input_clone

        struct #py_class_name {
            inner: #ident #generics
        }

        impl core::ops::Deref for #py_class_name {
            type Target = #ident #generics ;
            fn deref(&self) -> &Self::Target {
                &self.inner
            }
        }

        impl core::ops::DerefMut for #py_class_name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.inner
            }
        }
    };

    token
}
