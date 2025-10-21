use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{FnArg, ImplItem, ItemImpl, ReturnType, Token, punctuated::Punctuated};

use crate::{MacroArgs, StateMapping, generate_hardcoded_generics};

#[derive(PartialEq)]
enum SelfKind {
    None,
    Consume,
    ImmutableReference,
    MutableReference,
    Mutable,
}

impl ToTokens for SelfKind {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::None => {}
            Self::Consume => tokens.extend(quote! {self}),
            Self::ImmutableReference => tokens.extend(quote! {&self}),
            Self::MutableReference => tokens.extend(quote! {&mut self}),
            Self::Mutable => tokens.extend(quote! {mut self}),
        }
    }
}

pub fn macro_impl(args: super::MacroArgs, item: ItemImpl) -> TokenStream {
    let MacroArgs {
        visibility: _,
        py_class_name,
        state_mappings,
    } = args;
    let item_clone = item.clone();

    let unsafe_ = if let Some(_) = item.unsafety {
        quote! {unsafe}
    } else {
        TokenStream::new()
    };

    let trait_ = if let Some((_, path, _)) = item.trait_ {
        quote! {#path for}
    } else {
        TokenStream::new()
    };

    let new_impl_items: Vec<TokenStream> = item
        .items
        .iter()
        .map(|impl_item| generate_impl_item_block(&state_mappings, impl_item))
        .collect();

    quote! {
        #item_clone

        #[pyo3::pymethods]
        #unsafe_ impl #trait_ #py_class_name {
            #(#new_impl_items)*
        }
    }
}

// TODO: Support defaultness in impl item which is in feature specialization, https://github.com/rust-lang/rfcs/blob/master/text/1210-impl-specialization.md
fn generate_impl_item_block(state_mappings: &StateMapping, item: &ImplItem) -> TokenStream {
    match item {
        ImplItem::Const(item_const) => {
            let attrs = &item_const.attrs;
            let visibility = &item_const.vis;
            let ident = &item_const.ident;
            let ty = &item_const.ty;
            let expr = &item_const.expr;

            quote! {
                #(#attrs)*
                #visibility const #ident : #ty = #expr ;
            }
        }
        ImplItem::Type(item_type) => {
            let attrs = &item_type.attrs;
            let visibility = &item_type.vis;
            let ident = &item_type.ident;
            let ty = &item_type.ty;
            let generics = generate_hardcoded_generics(state_mappings, &item_type.generics);

            quote! {
                #(#attrs)*
                #visibility type #ident = #ty #generics ;
            }
        }
        ImplItem::Fn(item_fn) => {
            let attrs = &item_fn.attrs;
            let visibility = &item_fn.vis;
            let sig = &item_fn.sig;

            let unsafety = sig.unsafety;
            let constness = sig.constness;
            let asyncness = sig.asyncness;
            let abi = &sig.abi;

            let ident = &sig.ident;
            let (self_kind, new_input) = replace_generics_in_inputs(state_mappings, &sig.inputs);
            let new_return = replace_generics_in_return_type(state_mappings, &sig.output);

            if self_kind == SelfKind::None {
                // TODO: Better error message
                return syn::Error::new_spanned(
                    &sig.inputs,
                    "Functions without self aren't allowed in impl, please move them out",
                )
                .into_compile_error();
            }

            quote! {
                #(#attrs)*
                #visibility #unsafety #constness #asyncness #abi fn #ident (#self_kind, #new_input) #new_return {
                    self.inner. #ident (#new_input)
                }
            }
        }
        // TODO: Maybe give a full detail why?
        ImplItem::Macro(item_macro) => syn::Error::new_spanned(
            &item_macro,
            "Macros are currently unsupported by pyo3_state_machine",
        )
        .into_compile_error(),
        _ => panic!(
            "Unexpected error. Got ImplItem::Verbatim, please file an issue at https://github.com/AS1100K/pyo3_state_machine"
        ),
    }
}

fn replace_generics_in_inputs(
    _state_mappings: &StateMapping,
    inputs: &Punctuated<FnArg, Token![,]>,
) -> (SelfKind, TokenStream) {
    let mut self_kind = SelfKind::None;
    let mut new_inputs: Punctuated<FnArg, Token![,]> = Punctuated::new();

    for arg in inputs {
        match arg {
            FnArg::Receiver(rx) => {
                self_kind = match (&rx.reference, rx.mutability) {
                    (Some(_), Some(_)) => SelfKind::MutableReference,
                    (None, Some(_)) => SelfKind::Mutable,
                    (Some(_), None) => SelfKind::ImmutableReference,
                    (None, None) => SelfKind::Consume,
                };
            }
            FnArg::Typed(ty) => {
                let new_ty = ty.clone();

                // TODO: Update the generics with hardcoded types
                new_inputs.push(FnArg::Typed(new_ty));
            }
        }
    }

    (self_kind, new_inputs.to_token_stream())
}

fn replace_generics_in_return_type(
    _state_mappings: &StateMapping,
    return_type: &ReturnType,
) -> TokenStream {
    let new_return = return_type.clone();

    // TODO: Update the generics with hardcoded types
    new_return.into_token_stream()
}
