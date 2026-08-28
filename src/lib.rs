//! Element-wise tuple-ization for traits.
//!
//! `#[auto_tuple]` generates, for each requested arity, a helper trait whose
//! methods/consts/types forward element-wise to each element of an N-tuple:
//! `(x, y).foo(a)` desugars to `(x.foo(a), y.foo(a))`. See `docs/design.md`
//! for the full semantics.
//!
//! ```rust
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
//! // No import needed: the helper trait lives in this module.
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
    let track =
        analyze::decide_track(&trait_, &selected.methods, &selected.consts, &selected.types);

    let mut generated = Vec::new();
    let vis = cfg.vis.clone().unwrap_or_else(|| trait_.vis.clone());
    for n in cfg.sizes.clone() {
        generated.push(codegen::generate(&trait_, track, &selected, n, &vis, cfg.name.as_deref())?);
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

#[cfg(test)]
mod proptests {
    use proc_macro2::TokenStream;
    use proptest::prelude::*;

    use super::expand;

    /// Identifiers that may collide with generated names (`__T0`, `__TA0`,
    /// `__arg0`, ...) or with ordinary user names.
    fn ident_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            "__T0", "__T1", "__TA0", "__TA1", "__L0_a", "__C0_N", "__arg0", "T", "U", "N", "a",
            "foo", "bar", "x", "y",
        ]
    }

    proptest! {
        /// Any syntactically valid trait — including names that collide with
        /// the macro's generated identifiers — must expand without panicking
        /// (errors as `compile_error!` are fine).
        #[test]
        fn arbitrary_trait_never_panics(
            trait_name in ident_strategy(),
            method_names in prop::collection::vec(ident_strategy(), 1..5),
            generic_names in prop::collection::vec(ident_strategy(), 0..3),
        ) {
            let generics = generic_names.join(", ");
            let methods = method_names
                .iter()
                .map(|m| {
                    if generics.is_empty() {
                        format!("fn {m}(&self) -> usize;")
                    } else {
                        format!("fn {m}<{generics}>(x: {generic_names_0}) -> usize;", generic_names_0 = generic_names[0])
                    }
                })
                .collect::<String>();
            let src = format!("trait {trait_name} {{ {methods} }}");
            let tokens: TokenStream = src.parse().unwrap();
            let _ = expand(TokenStream::new(), tokens);
        }
    }
}
