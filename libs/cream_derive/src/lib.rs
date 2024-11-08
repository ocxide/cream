mod error;
mod gen_from_context;
mod common {
    use proc_macro2::TokenStream;

    // Only used in tests so it k
    pub fn streams_equal(a: &TokenStream, b: &TokenStream) -> bool {
        a.to_string() == b.to_string()
    }
}

use proc_macro::TokenStream;

#[proc_macro_derive(FromContext, attributes(context))]
pub fn from_context_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);
    gen_from_context::gen_from_context(ast).into()
}
