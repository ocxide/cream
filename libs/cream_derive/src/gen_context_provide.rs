use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::spanned::Spanned;

use crate::error::{span_compile_error, CompileError};

pub fn gen_context_provide(input: syn::DeriveInput) -> TokenStream {
    let global_span = input.span();
    let ast = match input.data {
        syn::Data::Struct(data) => data,
        _ => return span_compile_error!(global_span => "expected struct").into(),
    };

    let Some(attr) = input.attrs.into_iter().next() else {
        return span_compile_error!(global_span => "Missing #[provider_context] attribute").into();
    };

    let ContextProvideAttr { ident: ctx_ident } = match ContextProvideAttr::parse(attr) {
        Ok(attr) => attr,
        Err(err) => return CompileError::from(err).into(),
    };

    let struct_name = &input.ident;

    let build_tokens = match ast.fields {
        syn::Fields::Named(fields) => {
            let mappings = fields.named.iter().map(|field| {
                let mapping = quote! { self.ctx_provide() };
                let name = field.ident.as_ref().expect("expected named field");

                quote! { #name: #mapping }
            });

            quote! { #struct_name { #(#mappings),* } }
        }
        syn::Fields::Unnamed(fields) => {
            let mappings = fields.unnamed.iter().map(|_| {
                let mapping = quote! { self.ctx_provide() };
                quote! { #mapping }
            });

            quote! { #struct_name ( #(#mappings),* ) }
        }
        syn::Fields::Unit => {
            quote! { #struct_name }
        }
    };

    quote! {
        impl ContextProvide<#struct_name> for #ctx_ident {
            fn ctx_provide(&self) -> #struct_name {
                #build_tokens
            }
        }
    }
}

#[derive(Debug, PartialEq)]
struct ContextProvideAttr {
    ident: syn::Ident,
}

#[derive(Debug)]
struct Error {
    span: Span,
    kind: ErrorKind,
}

#[cfg(test)]
impl PartialEq<Self> for Error {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}

#[derive(Debug, PartialEq, Eq)]
enum ErrorKind {
    InvalidArg,
    TooManyArgs,
}

impl From<Error> for CompileError {
    fn from(value: Error) -> Self {
        match value.kind {
            ErrorKind::InvalidArg => {
                span_compile_error!(value.span => "expected #[provider_context(ident)]")
            }
            ErrorKind::TooManyArgs => {
                span_compile_error!(value.span => "expected one single argument")
            }
        }
    }
}

impl ContextProvideAttr {
    fn parse(attr: syn::Attribute) -> Result<Self, Error> {
        let attr_span = attr.span();
        let tokens = match attr.meta {
            syn::Meta::List(syn::MetaList { tokens, .. }) => tokens,
            _ => {
                return Err(Error {
                    span: attr_span,
                    kind: ErrorKind::InvalidArg,
                })
            }
        };

        let mut tokens = tokens.into_iter();
        let ident = match tokens.next() {
            Some(proc_macro2::TokenTree::Ident(ident)) => ident,
            _ => {
                return Err(Error {
                    span: attr_span,
                    kind: ErrorKind::InvalidArg,
                })
            }
        };

        if tokens.next().is_some() {
            return Err(Error {
                span: attr_span,
                kind: ErrorKind::TooManyArgs,
            });
        }

        Ok(Self { ident })
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn parses_attr() {
        let attr: syn::Attribute = parse_quote!(#[provider_context(C)]);
        assert_eq!(
            ContextProvideAttr::parse(attr),
            Ok(ContextProvideAttr {
                ident: parse_quote!(C)
            })
        );
    }

    #[test]
    fn parses_normal() {
        let input = quote! {
            #[provider_context(MyCtx)]
            struct Foo {
                bar: String,
                baz: usize,
            }
        };

        let ast: syn::DeriveInput = syn::parse2(input).unwrap();
        // dbg!(&ast.attrs);
        assert_eq!(
            super::gen_context_provide(ast).to_string(),
            quote! {
                impl ContextProvide<Foo> for MyCtx {
                    fn ctx_provide(&self) -> Foo {
                        Foo {
                            bar: self.ctx_provide(),
                            baz: self.ctx_provide()
                        }
                    }
                }
            }
            .to_string()
        );
    }
}
