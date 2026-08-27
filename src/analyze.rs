//! Track selection: decide between the shared and the All helper-trait shapes.
//!
//! The decision is based on whether any *selected* item's signature references
//! an original trait generic parameter. Method-local generic parameters are
//! excluded (the compiler forbids a method generic param that shadows a trait
//! generic param, E0403, so this exclusion is defensive only).

use std::collections::BTreeSet;

use syn::visit::Visit;
use syn::{GenericParam, ItemTrait, TraitItem};

/// Helper-trait shape to generate for a trait.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Track {
    /// Selected items reference original trait generic parameters; those
    /// parameters are lifted into the helper trait and shared by all elements.
    Shared,
    /// No original generic parameter is referenced; each element may implement
    /// `Tr` with its own parameters (element-wise independent).
    All,
}

/// Decides the track for a trait given the selected items.
pub fn decide_track(trait_: &ItemTrait, selected: &[&TraitItem]) -> Track {
    let trait_params = generic_param_names(&trait_.generics);

    // The original trait-level constraints must remain expressible in the
    // helper trait: parameter bounds and the where clause count as references.
    // Only the *bounds* are scanned; a generic parameter's own declaration is
    // not a reference (a `LifetimeParam` would otherwise trip `visit_lifetime`).
    let mut scanner =
        Scanner { trait_params: &trait_params, method_params: &BTreeSet::new(), hit: false };
    for param in &trait_.generics.params {
        if let GenericParam::Type(tp) = param {
            for bound in &tp.bounds {
                scanner.visit_type_param_bound(bound);
            }
            if let Some((_, default)) = &tp.default {
                scanner.visit_type(default);
            }
        }
    }
    if let Some(wc) = &trait_.generics.where_clause {
        scanner.visit_where_clause(wc);
    }
    if scanner.hit {
        return Track::Shared;
    }

    for item in selected {
        if item_references_generics(item, &trait_params) {
            return Track::Shared;
        }
    }

    // The All track only supports a single type generic parameter: its
    // per-element parameters are plain type params (`Tr<__TA{i}>`). Traits
    // with multiple params, lifetimes or const params are conservatively
    // downgraded to the shared track, which lifts the original params as-is.
    match trait_.generics.params.len() {
        1 if matches!(trait_.generics.params.first(), Some(GenericParam::Type(_))) => Track::All,
        _ => Track::Shared,
    }
}

/// Whether the given trait item's signature references any original trait
/// generic parameter, excluding method-local shadowing parameters.
fn item_references_generics(item: &TraitItem, trait_params: &BTreeSet<String>) -> bool {
    match item {
        TraitItem::Fn(f) => {
            let method_params = generic_param_names(&f.sig.generics);
            let mut scanner = Scanner { trait_params, method_params: &method_params, hit: false };
            scanner.visit_signature(&f.sig);
            scanner.hit
        }
        TraitItem::Const(c) => {
            let mut scanner = Scanner { trait_params, method_params: &BTreeSet::new(), hit: false };
            scanner.visit_type(&c.ty);
            scanner.hit
        }
        TraitItem::Type(t) => {
            let mut scanner = Scanner { trait_params, method_params: &BTreeSet::new(), hit: false };
            for bound in &t.bounds {
                scanner.visit_type_param_bound(bound);
            }
            scanner.hit
        }
        _ => false,
    }
}

fn generic_param_names(generics: &syn::Generics) -> BTreeSet<String> {
    generics.params.iter().map(param_name).collect()
}

fn param_name(param: &GenericParam) -> String {
    match param {
        GenericParam::Type(p) => p.ident.to_string(),
        GenericParam::Lifetime(p) => p.lifetime.ident.to_string(),
        GenericParam::Const(p) => p.ident.to_string(),
    }
}

struct Scanner<'a> {
    trait_params: &'a BTreeSet<String>,
    method_params: &'a BTreeSet<String>,
    hit: bool,
}

impl<'ast> Visit<'ast> for Scanner<'_> {
    fn visit_path(&mut self, node: &'ast syn::Path) {
        if let Some(seg) = node.segments.first() {
            let name = seg.ident.to_string();
            if !self.method_params.contains(&name) && self.trait_params.contains(&name) {
                self.hit = true;
            }
        }
        syn::visit::visit_path(self, node);
    }

    fn visit_lifetime(&mut self, node: &'ast syn::Lifetime) {
        // `&'a str` in a signature references the trait's `'a`; lifetimes are
        // not paths, so they need a dedicated hook.
        let name = node.ident.to_string();
        if !self.method_params.contains(&name) && self.trait_params.contains(&name) {
            self.hit = true;
        }
        syn::visit::visit_lifetime(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    fn select_one<'a>(trait_: &'a ItemTrait, name: &str) -> Vec<&'a TraitItem> {
        trait_.items.iter().filter(|item| trait_item_name(item).as_deref() == Some(name)).collect()
    }

    fn trait_item_name(item: &TraitItem) -> Option<String> {
        match item {
            TraitItem::Fn(f) => Some(&f.sig.ident),
            TraitItem::Const(c) => Some(&c.ident),
            TraitItem::Type(t) => Some(&t.ident),
            _ => None,
        }
        .map(|i| i.to_string())
    }

    fn decided(trait_: &ItemTrait, selected: &[&TraitItem]) -> Track {
        decide_track(trait_, selected)
    }

    #[test]
    fn method_using_trait_param_is_shared() {
        let trait_: ItemTrait = parse_quote! {
            trait Tr<T> {
                fn g(x: &T);
            }
        };
        let selected = select_one(&trait_, "g");
        assert_eq!(decided(&trait_, &selected), Track::Shared);
    }

    #[test]
    fn method_generic_same_name_is_all() {
        // Defensive: the compiler rejects a method generic param shadowing a
        // trait generic param (E0403), but the scanner must still not treat
        // the method's own param as a reference.
        let trait_: ItemTrait = parse_quote! {
            trait Tr<T> {
                fn g<T>(x: T);
            }
        };
        let selected = select_one(&trait_, "g");
        assert_eq!(decided(&trait_, &selected), Track::All);
    }

    #[test]
    fn method_without_trait_param_is_all() {
        let trait_: ItemTrait = parse_quote! {
            trait Tr<T> {
                fn foo(&self) -> Box<Self>;
            }
        };
        let selected = select_one(&trait_, "foo");
        assert_eq!(decided(&trait_, &selected), Track::All);
    }

    #[test]
    fn const_of_trait_param_type_is_shared() {
        let trait_: ItemTrait = parse_quote! {
            trait Tr<T> {
                const X: T;
            }
        };
        let selected = select_one(&trait_, "X");
        assert_eq!(decided(&trait_, &selected), Track::Shared);
    }

    #[test]
    fn assoc_type_bound_referencing_trait_param_is_shared() {
        let trait_: ItemTrait = parse_quote! {
            trait Tr<T> {
                type Out: AsRef<T>;
            }
        };
        let selected = select_one(&trait_, "Out");
        assert_eq!(decided(&trait_, &selected), Track::Shared);
    }

    #[test]
    fn trait_where_clause_referencing_param_is_shared() {
        let trait_: ItemTrait = parse_quote! {
            trait Tr<T>
            where
                T: Clone,
            {
                fn foo(&self) -> Box<Self>;
            }
        };
        let selected = select_one(&trait_, "foo");
        assert_eq!(decided(&trait_, &selected), Track::Shared);
    }

    #[test]
    fn mixed_shadowed_and_referencing_is_shared() {
        let trait_: ItemTrait = parse_quote! {
            trait Tr<T> {
                fn g<T>(x: T);
                fn h(x: T);
            }
        };
        let selected = vec![select_one(&trait_, "g").remove(0), select_one(&trait_, "h").remove(0)];
        assert_eq!(decided(&trait_, &selected), Track::Shared);
    }

    #[test]
    fn lifetime_reference_is_shared() {
        let trait_: ItemTrait = parse_quote! {
            trait Tr<'a> {
                fn f(x: &'a str);
            }
        };
        let selected = select_one(&trait_, "f");
        assert_eq!(decided(&trait_, &selected), Track::Shared);
    }

    #[test]
    fn method_local_lifetime_is_not_reference() {
        // A lifetime param disqualifies the All track, so the trait is
        // downgraded to shared even though the method never uses `'a`.
        let trait_: ItemTrait = parse_quote! {
            trait Tr<'a> {
                fn f<'b>(x: &'b str);
            }
        };
        let selected = select_one(&trait_, "f");
        assert_eq!(decided(&trait_, &selected), Track::Shared);
    }

    #[test]
    fn const_generic_reference_is_shared() {
        let trait_: ItemTrait = parse_quote! {
            trait Tr<const N: usize> {
                fn arr() -> [u8; N];
            }
        };
        let selected = select_one(&trait_, "arr");
        assert_eq!(decided(&trait_, &selected), Track::Shared);
    }

    #[test]
    fn trait_without_generics_is_shared_shape() {
        // No generic params: the All track is unavailable, so the decision is
        // shared; the generated shape is identical (plain name, no `__TA`).
        let trait_: ItemTrait = parse_quote! {
            trait Tr {
                fn foo(&self);
            }
        };
        let selected = select_one(&trait_, "foo");
        assert_eq!(decided(&trait_, &selected), Track::Shared);
    }

    #[test]
    fn multi_type_params_downgrade_to_shared() {
        let trait_: ItemTrait = parse_quote! {
            trait Tr<X, Y> {
                fn foo(&self);
            }
        };
        let selected = select_one(&trait_, "foo");
        assert_eq!(decided(&trait_, &selected), Track::Shared);
    }

    #[test]
    fn const_param_downgrades_to_shared() {
        let trait_: ItemTrait = parse_quote! {
            trait Tr<const N: usize> {
                fn foo(&self);
            }
        };
        let selected = select_one(&trait_, "foo");
        assert_eq!(decided(&trait_, &selected), Track::Shared);
    }

    #[test]
    fn single_type_param_stays_all() {
        let trait_: ItemTrait = parse_quote! {
            trait Tr<T> {
                fn foo(&self);
            }
        };
        let selected = select_one(&trait_, "foo");
        assert_eq!(decided(&trait_, &selected), Track::All);
    }
}
