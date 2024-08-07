use error::CompileError;
use gen_from_context::gen_from_context;
use proc_macro::TokenStream;
use quote::quote_spanned;
use syn::spanned::Spanned;

mod common {
    use proc_macro2::TokenStream;

    // Only used in tests so it k
    pub fn streams_equal(a: &TokenStream, b: &TokenStream) -> bool {
        a.to_string() == b.to_string()
    }
}

mod error;

mod impl_bound;
mod gen_from_context;

#[proc_macro_derive(FromContext, attributes(from_context))]
pub fn from_context_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);

    let global_span = ast.span();
    let Some(attr) = ast.attrs.into_iter().next() else {
        return quote_spanned! {
            global_span => compile_error!("Missing #[from_context] attribute")
        }
        .into();
    };

    let bound = match impl_bound::ImplBound::parse(attr) {
        Ok(bound) => bound,
        Err(err) => return CompileError::from(err).into(),
    };

    let struct_name = ast.ident;

    gen_from_context(&struct_name, ast.data, bound).into()
}
