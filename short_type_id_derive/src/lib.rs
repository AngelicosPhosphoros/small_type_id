#![allow(clippy::uninlined_format_args, clippy::missing_panics_doc)]

//! This crate implements derive proc_macro for crate `short_type_id`.
//! It is intended to be used through `short_type_id::HasTypeId` reexport.

use proc_macro::{Ident, Span, TokenStream, TokenTree};

/// Implements [`short_type_id::HasTypeId`](trait.HasTypeId.html) trait and registers implementation for runtime verification.
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

    let template = r#"
        #[allow(unsafe_code)]
        const _: () = {
            #[automatically_derived]
            unsafe impl ::short_type_id::HasTypeId for $$$$$ {
                const TYPE_ID: ::short_type_id::TypeId = {
                    const CRATE_VERSION: &::core::primitive::str = match ::core::option_env!("CARGO_PKG_VERSION") {
                        ::core::option::Option::Some(x) => x,
                        ::core::option::Option::None => "",
                    };
                    if CRATE_VERSION.is_empty() {
                        ::short_type_id::private::compute_id(
                            ::core::concat!(::core::module_path!(), "::", "$#$#$").as_bytes()
                        )
                    } else {
                        const SUM_LEN: usize =
                            ::core::concat!(::core::module_path!(), "::", "$#$#$", "::").len() + CRATE_VERSION.len();
                        let concatenated: [u8; SUM_LEN] = ::short_type_id::private::concat_bytes(
                            ::core::concat!(::core::module_path!(), "::", "$#$#$", "::").as_bytes(),
                            CRATE_VERSION.as_bytes(),
                        );
                        ::short_type_id::private::compute_id(&concatenated)
                    }
                };
            }

            static ENTRY_0KKVMQVJV2BRIOQ8EILZ7: ::short_type_id::private::TypeEntry = ::short_type_id::private::TypeEntry::new(
                ::core::concat!(::core::module_path!(), "::", "$#$#$"),
                <$$$$$ as ::short_type_id::HasTypeId>::TYPE_ID,
            );

            ::short_type_id::private::ctor!{
                #[ctor]
                #[inline]
                unsafe fn register_0kkvmqvjv2brioq8eilz7() {
                    unsafe {
                        ::short_type_id::private::register_type(&ENTRY_0KKVMQVJV2BRIOQ8EILZ7);
                    }
                }
            }
            ()
        };
    "#;

    let str_type_name = type_name.to_string();
    let without_prefix = str_type_name.strip_prefix("r#").unwrap_or(&str_type_name);
    let generated = template
        .replace("$$$$$", &str_type_name)
        .replace("$#$#$", without_prefix);
    let span = type_name.span();
    generated
        .parse::<TokenStream>()
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
