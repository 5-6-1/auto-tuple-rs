//! Element-wise tuple-ization for traits.
//!
//! `#[auto_tuple]` generates, for each requested arity, a helper trait whose
//! methods/consts/types forward element-wise to each element of an N-tuple:
//! `(x, y).foo(a)` desugars to `(x.foo(a), y.foo(a))`. See `docs/design.md`
//! for the full semantics.
//!
//! ```ignore
//! use auto_tuple::auto_tuple;
//!
//! #[auto_tuple]
//! trait Tr {
//!     fn zero() -> usize;
//! }
//!
//! struct X;
//! struct Y;
//!
//! impl Tr for X { fn zero() -> usize { 1 } }
//! impl Tr for Y { fn zero() -> usize { 2 } }
//!
//! # fn main() {
//! use auto_tuple::_TrTuple2;   // helper trait must be in scope
//! assert_eq!(<(X, Y)>::zero(), (1, 2));
//! # }
//! ```

mod analyze;
mod attr;
mod codegen;
mod rewrite;

use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemTrait, TraitItem};

use crate::attr::Config;
use crate::codegen::Selected;

/// Generates tuple helper traits for the annotated trait.
///
/// Arguments: an optional range of arities (default `2..=12`) and an optional
/// list of trait items to process (default: all methods; associated consts and
/// types are only processed when explicitly named).
#[proc_macro_attribute]
pub fn auto_tuple(args: TokenStream, input: TokenStream) -> TokenStream {
    match expand(args.into(), input.into()) {
        Ok(ts) => ts.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn expand(
    args: proc_macro2::TokenStream, input: proc_macro2::TokenStream,
) -> syn::Result<proc_macro2::TokenStream> {
    let cfg = Config::from_tokens(args)?;
    let trait_: ItemTrait = syn::parse2(input)?;

    if trait_.modifiers.auto_token.is_some() {
        return Err(syn::Error::new_spanned(
            &trait_,
            "auto traits are not supported by `#[auto_tuple]`",
        ));
    }

    let selected = select(&trait_, &cfg)?;
    let selected_refs: Vec<TraitItem> = selected
        .methods
        .iter()
        .map(|m| TraitItem::Fn((*m).clone()))
        .chain(selected.consts.iter().map(|c| TraitItem::Const((*c).clone())))
        .chain(selected.types.iter().map(|t| TraitItem::Type((*t).clone())))
        .collect();
    let track = analyze::decide_track(&trait_, &selected_refs.iter().collect::<Vec<_>>());

    let mut generated = Vec::new();
    for n in cfg.sizes.clone() {
        generated.push(codegen::generate(&trait_, track, &selected, n)?);
    }

    Ok(quote!(#trait_ #(#generated)*))
}

/// Picks the trait items to process.
///
/// Without an explicit list all methods are selected; associated consts and
/// types are only selected when explicitly named.
fn select<'a>(trait_: &'a ItemTrait, cfg: &Config) -> syn::Result<Selected<'a>> {
    let Some(names) = &cfg.items else {
        return Ok(Selected {
            methods: trait_
                .items
                .iter()
                .filter_map(|i| match i {
                    TraitItem::Fn(f) => Some(f),
                    _ => None,
                })
                .collect(),
            consts: Vec::new(),
            types: Vec::new(),
        });
    };

    let mut selected = Selected { methods: Vec::new(), consts: Vec::new(), types: Vec::new() };
    for name in names {
        let mut found = false;
        for item in &trait_.items {
            let ident = match item {
                TraitItem::Fn(f) => &f.sig.ident,
                TraitItem::Const(c) => &c.ident,
                TraitItem::Type(t) => &t.ident,
                _ => continue,
            };
            if ident == name {
                match item {
                    TraitItem::Fn(f) => selected.methods.push(f),
                    TraitItem::Const(c) => selected.consts.push(c),
                    TraitItem::Type(t) => selected.types.push(t),
                    _ => {}
                }
                found = true;
                break;
            }
        }
        if !found {
            return Err(syn::Error::new_spanned(trait_, format!("trait item `{name}` not found")));
        }
    }
    Ok(selected)
}
