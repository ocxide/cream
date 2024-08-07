use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Data, DataStruct, Field};

use crate::{error::span_compile_error, impl_bound::ImplBound};

pub fn gen_from_context(
    struct_name: &syn::Ident,
    struct_data: Data,
    impl_bound: ImplBound,
) -> TokenStream {
    let (impl_quote, context_ident) = match impl_bound {
        ImplBound::Static(ident) => (quote! { impl }, ident),
        ImplBound::Generic { ident, bounds } => (quote! { impl<#ident: #bounds> }, ident),
    };

    let build_tokens = match struct_data {
        Data::Struct(data) => gen_struct_build(data, &context_ident),
        _ => return span_compile_error!(struct_name.span() => "expected struct").into(),
    };

    quote! {
        #impl_quote FromContext<#context_ident> for #struct_name {
            fn from_context(ctx: &#context_ident) -> Self {
                #build_tokens
            }
        }
    }
}

fn gen_struct_build(
    DataStruct { fields, .. }: DataStruct,
    context_ident: &Ident,
) -> TokenStream {
    enum StructKind {
        KeyValued,    
        Tuple,
        Empty
    }

    let struct_kind = match fields
        .iter()
        .next() {
        Some(Field { ident: Some(_), .. }) => StructKind::KeyValued,
        Some(Field { ident: None, .. }) => StructKind::Tuple,
        None => StructKind::Empty
    };

    let field_mappings = fields.iter().map(|field| {
        let ty = &field.ty;
        let mapping = quote! { <#ty as FromContext<#context_ident>>::from_context(ctx) };

        match &field.ident {
            Some(name) => quote! { #name: #mapping },
            None => mapping,
        }
    });

    match struct_kind {
        StructKind::KeyValued => quote! { Self { #(#field_mappings),* } },
        StructKind::Tuple => quote! { Self ( #(#field_mappings),* ) },
        StructKind::Empty => quote! { Self },
    }
}

#[cfg(test)]
mod test {
    use proc_macro2::Span;

    use super::*;

    #[test]
    fn it_builds_key_structs() {
        let tokens = quote! {
            struct Foo {
                bar: String,
                baz: usize,
            }
        };

        let ast: syn::DeriveInput = syn::parse2(tokens).unwrap();
        let context_ident = Ident::new("C", Span::call_site());

        let struct_data = match ast.data {
            Data::Struct(data) => data,
            _ => unreachable!(),
        };

        let out_tokens = gen_struct_build(struct_data, &context_ident);
        assert_eq!(out_tokens.to_string(), quote! { Self { bar: <String as FromContext<C>>::from_context(ctx), baz: <usize as FromContext<C>>::from_context(ctx) }}.to_string());
    }

    #[test]
    fn it_builds_tuple_structs() {
        let tokens = quote! {
            struct Foo(String, usize);
        };

        let ast: syn::DeriveInput = syn::parse2(tokens).unwrap();
        let context_ident = Ident::new("C", Span::call_site());

        let struct_data = match ast.data {
            Data::Struct(data) => data,
            _ => unreachable!(),
        };

        let out_tokens = gen_struct_build(struct_data, &context_ident);
        assert_eq!(out_tokens.to_string(), quote! { Self(<String as FromContext<C>>::from_context(ctx), <usize as FromContext<C>>::from_context(ctx)) }.to_string());
    }

    #[test]
    fn it_builds_empty_structs() {
        let tokens = quote! {
            struct Foo;
        };

        let ast: syn::DeriveInput = syn::parse2(tokens).unwrap();
        let context_ident = Ident::new("C", Span::call_site());

        let struct_data = match ast.data {
            Data::Struct(data) => data,
            _ => unreachable!(),
        };

        let out_tokens = gen_struct_build(struct_data, &context_ident);
        assert_eq!(out_tokens.to_string(), quote! { Self }.to_string());
    }
}
