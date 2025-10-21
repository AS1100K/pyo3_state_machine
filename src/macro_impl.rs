use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{FnArg, Generics, Ident, ImplItem, ItemImpl, ReturnType, Token, punctuated::Punctuated};

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

    let original_type_ident = if let syn::Type::Path(type_path) = &*item.self_ty {
        type_path.path.segments.last().unwrap().ident.clone()
    } else {
        // Handle error or unsupported type
        panic!("Unsupported self type in impl block");
    };

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
        .map(|impl_item| {
            generate_impl_item_block(
                &state_mappings,
                impl_item,
                &py_class_name,
                &original_type_ident,
                &item.generics,
            )
        })
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
fn generate_impl_item_block(
    state_mappings: &StateMapping,
    item: &ImplItem,
    py_class_name: &Ident,
    original_type_ident: &Ident,
    original_type_generics: &Generics,
) -> TokenStream {
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
            let (self_kind, new_input, inner_call_input) =
                replace_generics_in_inputs(state_mappings, &sig.inputs, &py_class_name);
            let (new_return, wrapped_return) =
                replace_generics_in_return_type(state_mappings, &sig.output, &py_class_name);

            let return_statement = if wrapped_return {
                quote! {res.into()}
            } else {
                quote! {res}
            };

            let call_body = if self_kind == SelfKind::None {
                let hardcoded_generics =
                    generate_hardcoded_generics(state_mappings, original_type_generics);
                quote! {
                    let res = #original_type_ident :: #hardcoded_generics :: #ident (#inner_call_input) ;
                    #return_statement
                }
            } else {
                quote! {
                    let res = self.inner. #ident (#inner_call_input);
                    #return_statement
                }
            };

            quote! {
                #(#attrs)*
                #visibility #unsafety #constness #asyncness #abi fn #ident (#self_kind #new_input) #new_return {
                    #call_body
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
    state_mappings: &StateMapping,
    inputs: &Punctuated<FnArg, Token![,]>,
    py_class_name: &Ident,
) -> (SelfKind, TokenStream, TokenStream) {
    let mut self_kind = SelfKind::None;
    let mut new_inputs: Punctuated<FnArg, Token![,]> = Punctuated::new();
    let mut inner_inputs: Punctuated<TokenStream, Token![,]> = Punctuated::new();

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
                let (new_ty, _) =
                    replace_generics_in_type(state_mappings, &ty.ty, py_class_name, false);
                let mut new_ty_cloned = ty.clone();
                new_ty_cloned.ty = syn::parse2(new_ty).unwrap();
                new_inputs.push(FnArg::Typed(new_ty_cloned));
                inner_inputs.push(ty.pat.to_token_stream());
            }
        }
    }

    (
        self_kind,
        new_inputs.to_token_stream(),
        inner_inputs.to_token_stream(),
    )
}

fn replace_generics_in_return_type(
    state_mappings: &StateMapping,
    return_type: &ReturnType,
    py_class_name: &Ident,
) -> (TokenStream, bool) {
    match return_type {
        ReturnType::Default => (TokenStream::new(), false),
        ReturnType::Type(_, ty) => {
            let (new_ty, wrapped) =
                replace_generics_in_type(state_mappings, ty, py_class_name, true);
            (quote! { -> #new_ty }, wrapped)
        }
    }
}

fn replace_generics_in_type(
    state_mappings: &StateMapping,
    ty: &syn::Type,
    py_class_name: &Ident,
    wrap_return_type: bool,
) -> (TokenStream, bool) {
    let mut wrapped = false;
    let new_ty = match ty {
        syn::Type::Path(type_path) => {
            let mut new_type_path = type_path.clone();
            if let Some(last_segment) = new_type_path.path.segments.last_mut() {
                if last_segment.ident == "Self" {
                    wrapped = wrap_return_type;
                    last_segment.ident = py_class_name.clone();
                } else if let Some(mapped_type) =
                    state_mappings.get(&last_segment.ident.to_string())
                {
                    return (mapped_type.clone(), wrapped);
                }

                match &mut last_segment.arguments {
                    syn::PathArguments::AngleBracketed(angle_bracketed_args) => {
                        for arg in &mut angle_bracketed_args.args {
                            if let syn::GenericArgument::Type(arg_type) = arg {
                                let (replaced_type, _) = replace_generics_in_type(
                                    state_mappings,
                                    arg_type,
                                    py_class_name,
                                    wrap_return_type,
                                );
                                *arg_type = syn::parse2(replaced_type).unwrap();
                            }
                        }
                    }
                    _ => {}
                }
            }
            new_type_path.to_token_stream()
        }
        syn::Type::Reference(type_reference) => {
            let (replaced_type, _) = replace_generics_in_type(
                state_mappings,
                &type_reference.elem,
                py_class_name,
                wrap_return_type,
            );
            let lifetime = type_reference.lifetime.to_token_stream();
            let mutability = type_reference.mutability.to_token_stream();
            quote! { &#lifetime #mutability #replaced_type }
        }
        syn::Type::Tuple(type_tuple) => {
            let elems: Vec<TokenStream> = type_tuple
                .elems
                .iter()
                .map(|elem| {
                    let (replaced_type, _) = replace_generics_in_type(
                        state_mappings,
                        elem,
                        py_class_name,
                        wrap_return_type,
                    );
                    replaced_type
                })
                .collect();
            quote! { (#(#elems),*) }
        }
        syn::Type::Array(type_array) => {
            let (replaced_type, _) = replace_generics_in_type(
                state_mappings,
                &type_array.elem,
                py_class_name,
                wrap_return_type,
            );
            let len = &type_array.len;
            quote! { [#replaced_type; #len] }
        }
        _ => ty.to_token_stream(),
    };
    (new_ty, wrapped)
}
