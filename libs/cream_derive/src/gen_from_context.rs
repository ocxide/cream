use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::spanned::Spanned;

use crate::{
    common::streams_equal,
    error::{span_compile_error, CompileError},
};

pub fn gen_from_context(input: syn::DeriveInput) -> TokenStream {
    let global_span = input.span();
    let ast = match input.data {
        syn::Data::Struct(data) => data,
        _ => return span_compile_error!(global_span => "only structs are supported").into(),
    };

    let Some(attr) = input.attrs.into_iter().next() else {
        return span_compile_error!(global_span => "Missing #[context] attribute").into();
    };

    let FromContextAttr { context } = match FromContextAttr::parse(attr) {
        Ok(attr) => attr,
        Err(err) => return CompileError::from(err).into(),
    };

    let struct_name = &input.ident;

    let (ctx_name, header) = match context {
        ContextImpl::Static(context_name) => {
            let tokens = quote! { impl FromContext<#context_name> for #struct_name };
            (context_name, tokens)
        }

        ContextImpl::Generic { ident, bounds } => {
            let tokens = quote! { impl <#ident: #bounds> FromContext<#ident> for #struct_name };
            (ident, tokens)
        }
    };

    let build_tokens = match ast.fields {
        syn::Fields::Named(fields) => {
            let mappings = fields.named.iter().map(|field| {
                let ty = &field.ty;
                let mapping = quote! { <#ty as FromContext<#ctx_name>>::from_context(ctx) };
                let name = field.ident.as_ref().expect("expected named field");

                quote! { #name: #mapping }
            });

            quote! { Self { #(#mappings),* } }
        }
        syn::Fields::Unnamed(fields) => {
            let mappings = fields.unnamed.iter().map(|field| {
                let ty = &field.ty;
                let mapping = quote! { <#ty as FromContext<#ctx_name>>::from_context(ctx) };
                quote! { #mapping }
            });

            quote! { Self ( #(#mappings),* ) }
        }
        syn::Fields::Unit => {
            quote! { Self }
        }
    };

    quote! {
        #header {
            fn from_context(ctx: &#ctx_name) -> Self {
                #build_tokens
            }
        }
    }
}

#[derive(Debug, PartialEq)]
struct FromContextAttr {
    context: ContextImpl,
}

#[derive(Debug)]
enum ContextImpl {
    Static(Ident),
    Generic { ident: Ident, bounds: TokenStream },
}

impl PartialEq for ContextImpl {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Static(a), Self::Static(b)) => a == b,
            (
                Self::Generic {
                    ident: a,
                    bounds: a_bounds,
                },
                Self::Generic {
                    ident: b,
                    bounds: b_bounds,
                },
            ) => a == b && streams_equal(a_bounds, b_bounds),
            _ => false,
        }
    }
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
    InvalidMeta,
    NoContextIdent,
    InvalidContextIdent,
    InvalidBounds,
}

impl From<Error> for CompileError {
    fn from(err: Error) -> Self {
        match err.kind {
            ErrorKind::InvalidMeta => {
                span_compile_error!(err.span => "expected #[context(<ident|trait>)]")
            }
            ErrorKind::NoContextIdent => {
                span_compile_error!(err.span => "expected #[context(<ident|trait>)]")
            }
            ErrorKind::InvalidContextIdent => {
                span_compile_error!(err.span => "invalid context identifier")
            }
            ErrorKind::InvalidBounds => {
                span_compile_error!(err.span => "expected bounds like #[context(C: MyTrait + Foo + ...)]")
            }
        }
    }
}

impl FromContextAttr {
    fn parse(attr: syn::Attribute) -> Result<Self, Error> {
        let span = attr.span();
        let mut tokens = match attr.meta {
            syn::Meta::List(list) => list.tokens.into_iter(),
            _ => {
                return Err(Error {
                    kind: ErrorKind::InvalidMeta,
                    span,
                })
            }
        };

        let context_ident = match tokens.next() {
            Some(proc_macro2::TokenTree::Ident(context)) => context,
            None => {
                return Err(Error {
                    kind: ErrorKind::NoContextIdent,
                    span,
                })
            }
            _ => {
                return Err(Error {
                    kind: ErrorKind::InvalidContextIdent,
                    span,
                })
            }
        };

        let has_colon = match tokens.next() {
            Some(proc_macro2::TokenTree::Punct(colon)) if colon.as_char() == ':' => true,
            None => false,
            _ => {
                return Err(Error {
                    kind: ErrorKind::InvalidBounds,
                    span,
                })
            }
        };

        let tokens = tokens.collect::<TokenStream>();

        match (has_colon, tokens.is_empty()) {
            (false, true) => Ok(Self {
                context: ContextImpl::Static(context_ident),
            }),
            (true, false) => Ok(Self {
                context: ContextImpl::Generic {
                    ident: context_ident,
                    bounds: tokens,
                },
            }),
            _ => Err(Error {
                kind: ErrorKind::InvalidBounds,
                span: tokens.span(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn parses_generic() {
        let attr: syn::Attribute = parse_quote!(#[context(C: MyContext + 'static)]);
        assert_eq!(
            FromContextAttr::parse(attr).map(|attr| attr.context),
            Ok(ContextImpl::Generic {
                ident: parse_quote!(C),
                bounds: parse_quote!(MyContext + 'static),
            })
        );
    }

    #[test]
    fn parses_static() {
        let attr: syn::Attribute = parse_quote!(#[context(MyContext)]);
        assert_eq!(
            FromContextAttr::parse(attr).map(|attr| attr.context),
            Ok(ContextImpl::Static(parse_quote!(MyContext)))
        );
    }

    // Does not even matter, compiler catches it later
    /* #[test]
    fn detects_empty_bounds() {
        let attr: syn::Attribute = parse_quote!(#[from_context(C : &)]);
        assert_eq!(
            ImplBound::parse(attr.clone()),
            Err(Error {
                kind: ErrorKind::InvalidBounds,
                span: attr.span(),
            })
        );
    } */

    #[test]
    fn detects_no_context() {
        let attr: syn::Attribute = parse_quote!(#[context()]);
        assert_eq!(
            FromContextAttr::parse(attr.clone()).map(|attr| attr.context),
            Err(Error {
                kind: ErrorKind::NoContextIdent,
                span: attr.span(),
            })
        );
    }

    #[test]
    fn detects_invalid_context() {
        let attr: syn::Attribute = parse_quote!(#[context(1)]);
        assert_eq!(
            FromContextAttr::parse(attr.clone()).map(|attr| attr.context),
            Err(Error {
                kind: ErrorKind::InvalidContextIdent,
                span: attr.span(),
            })
        );
    }

    #[test]
    fn creates_static_impl() {
        let input: syn::DeriveInput = parse_quote!(
            #[context(MyContext)]
            struct Foo {
                bar: String,
                baz: usize,
            }
        );

        let result = quote! {
            impl FromContext<MyContext> for Foo {
                fn from_context (ctx: &MyContext) -> Self {
                    Self {
                        bar: ctx.ctx_provide(),
                        baz: ctx.ctx_provide()
                    }
                }
            }
        };

        assert_eq!(gen_from_context(input).to_string(), result.to_string(),);
    }
}
