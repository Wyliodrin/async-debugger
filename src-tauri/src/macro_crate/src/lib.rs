use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    Expr, ExprLit, ExprLoop, ItemFn, Lit, Meta, MetaNameValue, Stmt,
    fold::Fold,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    token::Comma,
};

struct Args {
    name: String,
    expr: Expr,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name = None;
        let mut expr = None;

        let metas: Punctuated<Meta, Comma> = input.parse_terminated(Meta::parse, Comma)?;
        for meta in metas {
            if let Meta::NameValue(MetaNameValue { path, value, .. }) = meta {
                if path.is_ident("name") {
                    if let Expr::Lit(ExprLit {
                        lit: Lit::Str(s), ..
                    }) = value
                    {
                        name = Some(s.value());
                    } else {
                        return Err(syn::Error::new_spanned(
                            path,
                            "expected `name = \"…\"` to be a string literal",
                        ));
                    }
                } else if path.is_ident("expr") {
                    expr = Some(value);
                }
            }
        }

        let name =
            name.ok_or_else(|| syn::Error::new(Span::call_site(), "missing `name = \"…\"`"))?;
        let expr = expr.ok_or_else(|| syn::Error::new(Span::call_site(), "missing `expr = …`"))?;

        Ok(Args { name, expr })
    }
}

/// Walk every `loop { … }` in the function body and append a debug‐send snippet at the end.
struct LoopInstrumenter {
    event_name: String,
    expr: Expr,
}

impl Fold for LoopInstrumenter {
    fn fold_expr_loop(&mut self, expr_loop: ExprLoop) -> ExprLoop {
        // Destructure the incoming loop
        let ExprLoop {
            attrs,
            label,
            loop_token,
            body,
            ..
        } = expr_loop;

        // Recursively fold any nested loops first
        let mut new_body = body;
        new_body = syn::fold::fold_block(self, new_body);

        // Build the snippet that serializes `self.expr` and sends it
        let name_lit = &self.event_name;
        let expr = &self.expr;
        let debug_send: Stmt = syn::parse2(quote! {
            {
                use chrono::Utc;
                use async_debugger::commands::debug_server::debug_proto::DebugEvent;

                let __dbg_payload_data = match serde_json::to_string(& ( #expr )) {
                    Ok(s)  => s,
                    Err(e) => {
                        eprintln!("debug_event payload serialize failed: {}", e);
                        String::new()
                    }
                };

                let __proto = DebugEvent {
                    name: #name_lit.to_string(),
                    kind: "loop".to_string(),
                    ts: Utc::now().timestamp_millis(),
                    payload: __dbg_payload_data,
                };
                let __client = __dbg_client.clone();
                tokio::spawn(async move {
                    if let Err(e) = __client.lock().await.send_event(__proto).await {
                        eprintln!("debug_event send in loop failed: {}", e);
                    }
                });
            }
        })
        .expect("failed to parse debug send snippet");

        new_body.stmts.push(debug_send);

        // Reconstruct the loop
        ExprLoop {
            attrs,
            label,
            loop_token,
            body: *Box::new(new_body),
            ..expr_loop
        }
    }
}

#[proc_macro_attribute]
pub fn debug_event(attr: TokenStream, item: TokenStream) -> TokenStream {
    let Args { name, expr } = parse_macro_input!(attr as Args);

    let mut func = parse_macro_input!(item as ItemFn);
    if func.sig.asyncness.is_none() {
        return syn::Error::new_spanned(&func.sig, "must be async")
            .to_compile_error()
            .into();
    }

    let orig_block = *func.block;
    let mut folder = LoopInstrumenter {
        event_name: name.clone(),
        expr,
    };
    let folded_block = folder.fold_block(orig_block);

    func.block = syn::parse2(quote!({
        let __dbg_client = match async_debugger::commands::debug_server::init_debug_client().await {
            Ok(c)  => c,
            Err(e) => { eprintln!("debug_event init failed: {}", e); return Err(e.into()); }
        };

        #folded_block
    }))
    .expect("failed to build wrapper block");

    TokenStream::from(quote! { #func })
}
