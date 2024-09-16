use proc_macro::TokenStream;

#[proc_macro_derive(ProtocolSchema)]
pub fn protocol_schema(input: TokenStream) -> TokenStream {
    helper::protocol_schema_impl(input)
}

mod helper {
    use proc_macro::TokenStream;

    pub fn protocol_schema_impl(_input: TokenStream) -> TokenStream {
        TokenStream::new()
    }
}
