use proc_macro::TokenStream;
use quote::quote;
use syn::{Ident, Token, parse::Parse, parse_macro_input, punctuated::Punctuated, token::Eq};

struct PtenvCallArgs {
    method: syn::Path,
    arguments: Vec<syn::Expr>,
    out_arg: Vec<syn::Expr>,
}

impl Parse for PtenvCallArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let method = input.parse::<syn::Path>()?;
        let _ = input.parse::<Token![,]>()?;

        let mut out_arg = Vec::new();
        let mut lookahead = input.lookahead1();
        while lookahead.peek(Ident) && input.peek(Ident) && input.peek2(Eq) {
            let ident: Ident = input.parse()?;
            if ident != "out" {
                return Err(syn::Error::new(ident.span(), "expected `out` keyword"));
            }
            let _eq: Eq = input.parse()?;
            let expr: syn::Expr = input.parse()?;
            out_arg.push(expr);
            let _ = input.parse::<Token![,]>()?;

            lookahead = input.lookahead1();
            assert!(out_arg.len() <= 2)
        }

        let args = Punctuated::<syn::Expr, Token![,]>::parse_terminated(input)?;
        Ok(PtenvCallArgs {
            method,
            arguments: args.into_iter().collect(),
            out_arg,
        })
    }
}

#[proc_macro]
pub fn ptenv_call(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as PtenvCallArgs);

    let method = &args.method;
    let arguments = &args.arguments;

    let registers = ["a0", "a1", "a2", "a3", "a4", "a5", "a6", "a7"];
    let mut asm_operands = Vec::new();

    for (i, arg) in arguments.iter().enumerate() {
        let reg = registers
            .get(i)
            .unwrap_or_else(|| panic!("Too many arguments (max {} supported)", registers.len()));
        asm_operands.push(quote! { in(#reg) #arg, });
    }

    let mut out_bindings = Vec::new();
    for (i, arg) in args.out_arg.iter().enumerate() {
        let reg = registers
            .get(i)
            .unwrap_or_else(|| panic!("Too many arguments (max {} supported)", registers.len()));
        out_bindings.push(quote! { lateout(#reg) #arg, });
    }

    quote! {
        unsafe {
            asm!(
                r"
                li t0, 1 << 1
                csrrc s2, sstatus, t0
                csrr s3, satp
        
                csrw satp, {token}
        
                mv s4, ra
                call {func}
        
                csrw satp, s3
                csrw sstatus, s2
                ",
                func = sym #method,
                token = in(reg) PTENV_TOKEN,
                #(#asm_operands)*
                #(#out_bindings)*
                options(nostack, preserves_flags),
            );
        }
    }
    .into()
}
