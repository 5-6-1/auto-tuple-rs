#![doc = include_str!("../README.md")]

mod analyze;
mod attr;
mod codegen;
mod rewrite;
mod selection;

use proc_macro::TokenStream;
use quote::quote;
use syn::ItemTrait;

use crate::attr::Config;

/// Generates tuple helper traits for the annotated trait.
///
/// Arguments: an optional range of arities (default `2..=12`) and an optional
/// member selection (default: all methods, associated constants and types).
/// Names and `@all` families form a union; `-name`, `-[a, b]` and `-@all_types`
/// exclude members. An exclusion-only selection starts from all members.
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

    let selected = selection::select(&trait_, &cfg.selection)?;
    let track =
        analyze::decide_track(&trait_, &selected.methods, &selected.consts, &selected.types);

    let mut generated = Vec::new();
    let vis = cfg.vis.clone().unwrap_or_else(|| trait_.vis.clone());
    for n in cfg.sizes.clone() {
        generated.push(codegen::generate(&trait_, track, &selected, n, &vis, cfg.name.as_deref())?);
    }

    Ok(quote!(#trait_ #(#generated)*))
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

    /// Trait generic-parameter shapes (all syntactically valid).
    fn trait_generics_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just(String::new()),
            Just("<T>".to_string()),
            Just("<T, U>".to_string()),
            Just("<'a>".to_string()),
            Just("<const N: usize>".to_string()),
            Just("<T: Clone>".to_string()),
            Just("<T = u32>".to_string()),
        ]
    }

    /// Method shapes covering receivers, args, generics, async, unsafe,
    /// `Self`-typed params/returns, RPIT and `where` clauses.
    fn method_strategy() -> impl Strategy<Value = &'static str> {
        prop::sample::select(vec![
            "fn foo(&self) -> usize;",
            "fn foo(&mut self, x: usize) -> usize;",
            "fn foo(self);",
            "fn foo();",
            "fn foo<X>(x: X) -> X;",
            "fn foo<T: Clone>(&self, x: T) -> T;",
            "fn foo(&self, x: &Self);",
            "fn foo(&mut self, x: &mut Self);",
            "fn foo(&self) -> Box<Self>;",
            "fn foo(&self) -> Vec<Self> where Self: Sized;",
            "async fn foo(&self) -> usize;",
            "unsafe fn foo(&self) -> usize;",
            "fn foo(&self) -> impl Iterator<Item = Self>;",
            "fn foo(&self, x: Self::Output) -> Self::Output;",
            "fn foo(&self) -> usize where Self: Sized;",
        ])
    }

    /// Associated const/type items (syntactically valid).
    fn extra_item_strategy() -> impl Strategy<Value = &'static str> {
        prop::sample::select(vec![
            "const MAX: usize;",
            "type Out;",
            "type Out: Clone;",
            "type Out = usize;",
        ])
    }

    proptest! {
        /// Any syntactically valid trait — including names that collide with
        /// the macro's generated identifiers and every supported signature
        /// shape — must expand without panicking (errors as `compile_error!`
        /// are fine; the point is "never a macro panic").
        #[test]
        fn arbitrary_trait_never_panics(
            trait_name in ident_strategy(),
            trait_generics in trait_generics_strategy(),
            methods in prop::collection::vec(method_strategy(), 0..6),
            extra_items in prop::collection::vec(extra_item_strategy(), 0..3),
            args in prop::sample::select(vec![
                "", "0..=1", "@all", "@all_methods, 0..=2", "@all_types",
                "@all_constants", "@all, -@all", "@all_ref_methods, -foo",
                "[@all_methods, Out], -foo", "@all_required", "@all_default",
                "@missing", "-", "[]",
            ]),
        ) {
            let body = [methods.join(" "), extra_items.join(" ")].join(" ");
            let src = format!("trait {trait_name}{trait_generics} {{ {body} }}");
            let tokens: TokenStream = src.parse().unwrap();
            // Keep the generator honest: a syntax error must not turn this
            // into a property test of the parser's early-return path alone.
            syn::parse2::<syn::ItemTrait>(tokens.clone()).unwrap();
            let _ = expand(args.parse().unwrap(), tokens);
        }
    }
}
