#![allow(clippy::uninlined_format_args, clippy::missing_panics_doc)]

//! # THIS IS ALPHA RELEASE PLEASE DO NOT USE
//! This crate implements derive `proc_macro` for crate `small_type_id`.
//! It is intended to be used through `small_type_id::HasTypeId` reexport.

use proc_macro::{Ident, Span, TokenStream, TokenTree};

/// Implements [`small_type_id::HasTypeId`](trait.HasTypeId.html) trait and registers implementation for runtime verification.
#[proc_macro_derive(HasTypeId)]
pub fn derive_has_type_id_trait(items: TokenStream) -> TokenStream {
    let type_name: Ident = 'type_name: {
        enum State {
            BeforeKeyBinding,
            HasKeyBinding,
            HasTypeName(Ident),
        }

        let mut state = State::BeforeKeyBinding;
        for token in items {
            match (&state, token) {
                (State::BeforeKeyBinding, TokenTree::Ident(ident)) => {
                    let s = ident.to_string();
                    if s == "struct" || s == "enum" || s == "union" {
                        state = State::HasKeyBinding;
                    }
                }
                // Other stuff for visibility.
                (State::BeforeKeyBinding, _) => {}
                (State::HasKeyBinding, TokenTree::Ident(type_name)) => {
                    state = State::HasTypeName(type_name);
                }
                (State::HasKeyBinding, _) => {
                    unreachable!("Rust grammar requires type name right after keyword")
                }
                (State::HasTypeName(_), TokenTree::Punct(punct)) if punct.as_char() == '<' => {
                    return make_compile_error("Generics are not supported", punct.span());
                }
                (State::HasTypeName(type_name), TokenTree::Punct(_) | TokenTree::Group(_)) => {
                    break 'type_name type_name.clone();
                }
                (State::HasTypeName(_), _) => unreachable!("Invalid Rust grammar"),
            }
        }
        unreachable!("Invalid rust syntax for user type declaration")
    };

    let name_str = type_name.to_string();
    let text = format!(
        "::small_type_id::private::implement_type_and_register!({}, \"{}\");",
        name_str,
        name_str.strip_prefix("r#").unwrap_or(&name_str),
    );

    let span = type_name.span();

    text.parse::<TokenStream>()
        .unwrap()
        .into_iter()
        .map(|mut t| {
            t.set_span(span);
            t
        })
        .collect()
}

fn make_compile_error(msg: &str, span: Span) -> TokenStream {
    format!(r#"::core::compile_error!("{}")"#, msg)
        .parse::<TokenStream>()
        .unwrap()
        .into_iter()
        .map(|mut t| {
            t.set_span(span);
            t
        })
        .collect()
}
