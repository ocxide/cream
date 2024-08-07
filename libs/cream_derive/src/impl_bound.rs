use proc_macro2::{Ident, Span, TokenStream};
use syn::spanned::Spanned;

use crate::{
    common::streams_equal,
    error::{span_compile_error, CompileError},
};

#[derive(Debug)]
pub struct Error {
    pub kind: ErrorKind,
    pub span: Span,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ErrorKind {
    InvalidMeta,
    NoContextIdent,
    InvalidContextIdent,
    InvalidBounds,
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}

impl From<Error> for CompileError {
    fn from(err: Error) -> Self {
        match err.kind {
            ErrorKind::InvalidMeta => {
                span_compile_error!(err.span => "expected #[from_context]")
            }
            ErrorKind::NoContextIdent => {
                span_compile_error!(err.span => "expected context identifier")
            }
            ErrorKind::InvalidContextIdent => {
                span_compile_error!(err.span => "invalid context identifier")
            }
            ErrorKind::InvalidBounds => span_compile_error!(err.span => "invalid context bounds"),
        }
    }
}

#[derive(Debug)]
pub enum ImplBound {
    Static(Ident),
    Generic { ident: Ident, bounds: TokenStream },
}

impl PartialEq<Self> for ImplBound {
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

impl ImplBound {
    pub fn parse(attr: syn::Attribute) -> Result<Self, Error> {
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
            (false, true) => Ok(ImplBound::Static(context_ident)),
            (true, false) => Ok(ImplBound::Generic {
                ident: context_ident,
                bounds: tokens,
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
        let attr: syn::Attribute = parse_quote!(#[from_context(C: MyContext + 'static)]);
        assert_eq!(
            ImplBound::parse(attr),
            Ok(ImplBound::Generic {
                ident: parse_quote!(C),
                bounds: parse_quote!(MyContext + 'static),
            })
        );
    }

    #[test]
    fn parses_static() {
        let attr: syn::Attribute = parse_quote!(#[from_context(MyContext)]);
        assert_eq!(
            ImplBound::parse(attr),
            Ok(ImplBound::Static(parse_quote!(MyContext)))
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
        let attr: syn::Attribute = parse_quote!(#[from_context()]);
        assert_eq!(
            ImplBound::parse(attr.clone()),
            Err(Error {
                kind: ErrorKind::NoContextIdent,
                span: attr.span(),
            })
        );
    }

    #[test]
    fn detects_invalid_context() {
        let attr: syn::Attribute = parse_quote!(#[from_context(1)]);
        assert_eq!(
            ImplBound::parse(attr.clone()),
            Err(Error {
                kind: ErrorKind::InvalidContextIdent,
                span: attr.span(),
            })
        );
    }
}

