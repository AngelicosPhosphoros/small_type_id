#![allow(clippy::uninlined_format_args, clippy::missing_panics_doc)]

//! # THIS IS ALPHA RELEASE PLEASE DO NOT USE
//! This crate implements derive `proc_macro` for crate `small_type_id`.
//! It is intended to be used through `small_type_id::HasTypeId` reexport.

use proc_macro::Spacing::Alone;
use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Span, TokenStream, TokenTree};

/// Implements [`small_type_id::HasTypeId`](trait.HasTypeId.html) trait and registers implementation for runtime verification.
#[proc_macro_derive(HasTypeId, attributes(small_type_id_seed))]
pub fn derive_has_type_id_trait(items: TokenStream) -> TokenStream {
    let (type_name, seed): (Ident, Literal) = 'type_name: {
        enum State {
            BeforeAttr,
            BeforeKeyWord,
            HasKeyWord,
            HasTypeName(Ident),
        }

        let mut seed = Literal::u32_suffixed(0);
        let mut state = State::BeforeAttr;
        let mut items = items.into_iter();
        while let Some(token) = items.next() {
            match (&state, token) {
                (State::BeforeAttr, TokenTree::Punct(p)) if p.as_char() == '#' => {
                    seed = match try_parse_initial_seed(&mut items, p.span()) {
                        Ok(Some(seed)) => seed,
                        Ok(None) => continue,
                        Err(err) => return err,
                    };
                    state = State::BeforeKeyWord;
                }
                (State::BeforeKeyWord | State::BeforeAttr, TokenTree::Ident(ident)) => {
                    let s = ident.to_string();
                    if s == "struct" || s == "enum" || s == "union" {
                        state = State::HasKeyWord;
                    }
                }
                // Other stuff for visibility.
                (State::BeforeKeyWord | State::BeforeAttr, _) => {}
                (State::HasKeyWord, TokenTree::Ident(type_name)) => {
                    state = State::HasTypeName(type_name);
                }
                (State::HasKeyWord, _) => {
                    unreachable!("Rust grammar requires type name right after keyword")
                }
                (State::HasTypeName(_), TokenTree::Punct(punct)) if punct.as_char() == '<' => {
                    return make_compile_error("Generics are not supported", punct.span());
                }
                (State::HasTypeName(type_name), TokenTree::Punct(_) | TokenTree::Group(_)) => {
                    break 'type_name (type_name.clone(), seed);
                }
                (State::HasTypeName(_), _) => unreachable!("Invalid Rust grammar"),
            }
        }
        unreachable!("Invalid rust syntax for user type declaration")
    };

    let span = type_name.span();
    let name_str = type_name.to_string();
    let non_raw_name = name_str.strip_prefix("r#").unwrap_or(&name_str);

    let invoke_start: TokenStream = "::small_type_id::private::implement_type_and_register!"
        .parse()
        .unwrap();

    invoke_start
        .into_iter()
        .map(|mut t| {
            t.set_span(span);
            t
        })
        .chain(
            // Chain with `(type_name, "type_name", number);`
            [
                TokenTree::Group(Group::new(
                    Delimiter::Parenthesis,
                    [
                        TokenTree::Ident(type_name),
                        TokenTree::Punct(Punct::new(',', Alone)),
                        TokenTree::Literal(Literal::string(non_raw_name)),
                        TokenTree::Punct(Punct::new(',', Alone)),
                        TokenTree::Literal(seed),
                    ]
                    .into_iter()
                    .collect(),
                )),
                TokenTree::Punct(Punct::new(';', Alone)),
            ],
        )
        .collect()
}

fn make_compile_error(msg: &str, span: Span) -> TokenStream {
    format!(r#"::core::compile_error!("{}");"#, msg)
        .parse::<TokenStream>()
        .unwrap()
        .into_iter()
        .map(|mut t| {
            t.set_span(span);
            t
        })
        .collect()
}

/// # Returns
/// None if it was different attribute.
/// Some(literal) if it was `small_type_id_seed`
/// # Errors
/// If invalid attribute.
fn try_parse_initial_seed(
    token_stream: &mut impl Iterator<Item = TokenTree>,
    span: Span,
) -> Result<Option<Literal>, TokenStream> {
    let tt = token_stream.next();
    let span = tt.as_ref().map_or(span, TokenTree::span);
    let mk_err = move || {
        Err(make_compile_error(
            "Correct format: `#[small_type_id_seed=number_u32]`",
            span,
        ))
    };
    let Some(TokenTree::Group(g)) = tt else {
        return Ok(None);
    };
    if g.delimiter() != Delimiter::Bracket {
        return Ok(None);
    }
    let mut it = g.stream().into_iter();
    let Some(TokenTree::Ident(ident)) = it.next() else {
        return Ok(None);
    };
    if ident.to_string() != "small_type_id_seed" {
        return Ok(None);
    }
    let Some(TokenTree::Punct(assignment)) = it.next() else {
        return mk_err();
    };
    if assignment.as_char() != '=' {
        return mk_err();
    }
    let Some(TokenTree::Literal(value)) = it.next() else {
        return mk_err();
    };
    if it.next().is_some() {
        return mk_err();
    }

    Ok(Some(value))
}
