use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    Expr, ExprLit, ItemFn, Lit, Meta, MetaNameValue,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    token::Comma,
};

struct Args(Punctuated<Meta, Comma>);
impl Parse for Args {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Args(input.parse_terminated(Meta::parse, Comma)?))
    }
}

fn parse_name_arg(metas: Punctuated<Meta, Comma>) -> syn::Result<String> {
    for meta in metas {
        if let Meta::NameValue(MetaNameValue { path, value, .. }) = meta {
            if path.is_ident("name") {
                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) = value
                {
                    return Ok(s.value());
                } else {
                    return Err(syn::Error::new_spanned(
                        path,
                        "expected `name = \"…\"` to be a string literal",
                    ));
                }
            }
        }
    }
    Err(syn::Error::new(
        Span::call_site(),
        "missing required `name = \"…\"` argument",
    ))
}

#[proc_macro_attribute]
pub fn debug_event(attr: TokenStream, item: TokenStream) -> TokenStream {
    let Args(metas) = parse_macro_input!(attr as Args);
    let name = match parse_name_arg(metas) {
        Ok(n) => n,
        Err(err) => return err.to_compile_error().into(),
    };

    let mut func = parse_macro_input!(item as ItemFn);

    if func.sig.asyncness.is_none() {
        return syn::Error::new_spanned(
            &func.sig,
            "`#[debug_event]` can only be applied to async fns",
        )
        .to_compile_error()
        .into();
    }

    let output_ty = match &func.sig.output {
        syn::ReturnType::Default => quote! { () },
        syn::ReturnType::Type(_, ty) => quote! { #ty },
    };

    let orig = &func.block;

    let instrumented = quote!({
        let __dbg_client = match async_debugger::commands::debug_server::init_debug_client().await {
            Ok(c)    => c,
            Err(e)   => { eprintln!("debug_event init failed: {}", e); return Err(e.into()); }
        };

        let __dbg_result: #output_ty = async move #orig.await;
        let debug_result = match &__dbg_result {
            Ok(v)  => async_debugger::commands::debug_server::DebugResult::Ok(),
            Err(e) => async_debugger::commands::debug_server::DebugResult::Err(e.to_string()),
        };
        let __dbg_payload = match serde_json::to_string(&debug_result) {
            Ok(s)  => s,
            Err(e) => { eprintln!("debug_event serialize failed: {}", e); String::new() }
        };

        {
            use chrono::Utc;
            use async_debugger::commands::debug_server::debug_proto::DebugEvent;
            let __proto = DebugEvent {
                name:    #name.to_string(),
                kind:    "mpsc".to_string(),
                ts:      Utc::now().timestamp_millis(),
                payload: __dbg_payload,
            };
            let __client = __dbg_client.clone();
            tokio::spawn(async move {
                if let Err(e) = __client.lock().await.send_event(__proto).await {
                    eprintln!("debug_event send failed: {}", e);
                }
            });
        }

        __dbg_result
    });

    func.block =
        syn::parse2(instrumented).expect("internal error: failed to parse instrumented block");

    TokenStream::from(quote! { #func })
}
