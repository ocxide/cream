mod error;
mod gen_context_provide;
mod common {
    use proc_macro2::TokenStream;

    // Only used in tests so it k
    pub fn streams_equal(a: &TokenStream, b: &TokenStream) -> bool {
        a.to_string() == b.to_string()
    }
}

use proc_macro::TokenStream;

#[proc_macro_derive(ContextProvide, attributes(provider_context))]
pub fn context_provide_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);
    gen_context_provide::gen_context_provide(ast).into()
}
