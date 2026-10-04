//! Member-set expressions and their resolution against the original trait.

use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Ident, ItemTrait, Result, Token, TraitItem, TraitItemConst, TraitItemFn, TraitItemType};

/// Positive terms are unioned before exclusions are applied. An absent positive
/// term means all members; a positive family matching nothing means an empty set.
#[derive(Clone, Default)]
pub struct Selection {
    include: Vec<Selector>,
    exclude: Vec<Selector>,
}

impl Selection {
    pub fn parse_term(&mut self, input: ParseStream) -> Result<()> {
        let exclude = input.parse::<Option<Token![-]>>()?.is_some();
        let terms = if input.peek(syn::token::Bracket) {
            let content;
            syn::bracketed!(content in input);
            let terms = Punctuated::<Selector, Token![,]>::parse_terminated(&content)?;
            if terms.is_empty() {
                return Err(content.error("selector lists must not be empty"));
            }
            terms.into_iter().collect()
        } else {
            vec![input.parse()?]
        };
        if exclude { &mut self.exclude } else { &mut self.include }.extend(terms);
        Ok(())
    }
}

#[derive(Clone)]
enum Selector {
    Name(Ident),
    Family(Family),
}

impl Parse for Selector {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.parse::<Option<Token![@]>>()?.is_some() {
            let name: Ident = input.parse()?;
            Family::named(&name.to_string()).map(Self::Family).ok_or_else(|| {
                syn::Error::new(name.span(), format!("unknown member selector `@{name}`"))
            })
        } else {
            input.parse().map(Self::Name)
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Method,
    Constant,
    Type,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Receiver {
    Ref,
    Value,
    Static,
}

/// The same member-family vocabulary as batch-impl. These are predicates on
/// original items, independent of whether code generation supports an item.
#[derive(Clone)]
struct Family {
    kind: Option<Kind>,
    default: Option<bool>,
    receiver: Option<Receiver>,
}

impl Family {
    fn named(name: &str) -> Option<Self> {
        let (kind, default, receiver) = match name {
            "all" => (None, None, None),
            "all_methods" => (Some(Kind::Method), None, None),
            "all_constants" => (Some(Kind::Constant), None, None),
            "all_types" => (Some(Kind::Type), None, None),
            "all_default" => (None, Some(true), None),
            "all_default_methods" => (Some(Kind::Method), Some(true), None),
            "all_default_constants" => (Some(Kind::Constant), Some(true), None),
            "all_default_types" => (Some(Kind::Type), Some(true), None),
            "all_required" => (None, Some(false), None),
            "all_required_methods" => (Some(Kind::Method), Some(false), None),
            "all_required_constants" => (Some(Kind::Constant), Some(false), None),
            "all_required_types" => (Some(Kind::Type), Some(false), None),
            "all_ref_methods" => (Some(Kind::Method), None, Some(Receiver::Ref)),
            "all_value_methods" => (Some(Kind::Method), None, Some(Receiver::Value)),
            "all_static_methods" => (Some(Kind::Method), None, Some(Receiver::Static)),
            _ => return None,
        };
        Some(Self { kind, default, receiver })
    }

    fn matches(&self, item: &TraitItem) -> bool {
        let (kind, default, receiver) = match item {
            TraitItem::Fn(f) => (
                Kind::Method,
                f.default.is_some(),
                match f.sig.receiver().map(|r| &r.kind) {
                    Some(syn::ReceiverKind::Reference(..)) => Some(Receiver::Ref),
                    Some(syn::ReceiverKind::Value | syn::ReceiverKind::Typed(..)) => {
                        Some(Receiver::Value)
                    }
                    None => Some(Receiver::Static),
                    Some(_) => None,
                },
            ),
            TraitItem::Const(c) => (Kind::Constant, c.default.is_some(), None),
            TraitItem::Type(t) => (Kind::Type, t.default.is_some(), None),
            _ => return false,
        };
        self.kind.is_none_or(|k| k == kind)
            && self.default.is_none_or(|d| d == default)
            && self.receiver.is_none_or(|r| Some(r) == receiver)
    }
}

impl Selector {
    fn matches(&self, item: &TraitItem) -> bool {
        match self {
            Self::Name(name) => item_name(item).is_some_and(|id| id.unraw() == name.unraw()),
            Self::Family(family) => family.matches(item),
        }
    }
}

fn item_name(item: &TraitItem) -> Option<&Ident> {
    match item {
        TraitItem::Fn(f) => Some(&f.sig.ident),
        TraitItem::Const(c) => Some(&c.ident),
        TraitItem::Type(t) => Some(&t.ident),
        _ => None,
    }
}

/// Selected items, in declaration order within each kind. Analysis and code
/// generation consume this resolved view rather than reinterpreting syntax.
pub struct Selected<'a> {
    pub methods: Vec<&'a TraitItemFn>,
    pub consts: Vec<&'a TraitItemConst>,
    pub types: Vec<&'a TraitItemType>,
}

pub fn select<'a>(trait_: &'a ItemTrait, selection: &Selection) -> Result<Selected<'a>> {
    for term in selection.include.iter().chain(&selection.exclude) {
        if let Selector::Name(name) = term
            && !trait_.items.iter().any(|item| term.matches(item))
        {
            return Err(syn::Error::new(name.span(), format!("trait item `{name}` not found")));
        }
    }

    let mut selected = Selected { methods: vec![], consts: vec![], types: vec![] };
    for item in &trait_.items {
        let included =
            selection.include.is_empty() || selection.include.iter().any(|term| term.matches(item));
        if !included || selection.exclude.iter().any(|term| term.matches(item)) {
            continue;
        }
        match item {
            TraitItem::Fn(f) => selected.methods.push(f),
            TraitItem::Const(c) => selected.consts.push(c),
            TraitItem::Type(t) => selected.types.push(t),
            _ => {}
        }
    }
    Ok(selected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attr::Config;
    use syn::parse_quote;

    fn fixture() -> ItemTrait {
        parse_quote! {
            trait Tr {
                fn read(&self);
                fn edit(&mut self) {}
                fn take(self);
                fn boxed(self: Box<Self>) {}
                fn new() -> Self;
                const REQUIRED: usize;
                const DEFAULT: usize = 1;
                type Output;
                type DefaultOutput = usize;
            }
        }
    }

    fn names(args: &str) -> Vec<String> {
        let cfg = Config::from_tokens(args.parse().unwrap()).unwrap();
        let trait_ = fixture();
        let selected = select(&trait_, &cfg.selection).unwrap();
        selected
            .methods
            .iter()
            .map(|f| f.sig.ident.to_string())
            .chain(selected.consts.iter().map(|c| c.ident.to_string()))
            .chain(selected.types.iter().map(|t| t.ident.to_string()))
            .collect()
    }

    #[test]
    fn member_families_filter_original_items() {
        for (selector, expected) in [
            (
                "@all",
                vec![
                    "read",
                    "edit",
                    "take",
                    "boxed",
                    "new",
                    "REQUIRED",
                    "DEFAULT",
                    "Output",
                    "DefaultOutput",
                ],
            ),
            ("@all_methods", vec!["read", "edit", "take", "boxed", "new"]),
            ("@all_constants", vec!["REQUIRED", "DEFAULT"]),
            ("@all_types", vec!["Output", "DefaultOutput"]),
            ("@all_required", vec!["read", "take", "new", "REQUIRED", "Output"]),
            ("@all_default", vec!["edit", "boxed", "DEFAULT", "DefaultOutput"]),
            ("@all_required_methods", vec!["read", "take", "new"]),
            ("@all_default_methods", vec!["edit", "boxed"]),
            ("@all_required_constants", vec!["REQUIRED"]),
            ("@all_default_constants", vec!["DEFAULT"]),
            ("@all_required_types", vec!["Output"]),
            ("@all_default_types", vec!["DefaultOutput"]),
            ("@all_ref_methods", vec!["read", "edit"]),
            ("@all_value_methods", vec!["take", "boxed"]),
            ("@all_static_methods", vec!["new"]),
        ] {
            assert_eq!(names(selector), expected, "{selector}");
        }
    }

    #[test]
    fn default_and_config_only_mean_all() {
        assert_eq!(names(""), names("@all"));
        assert_eq!(names("pub(crate), name = \"Chosen\", 2..=3"), names("@all"));
        assert_eq!(names("-read"), names("@all, -read"));
    }

    #[test]
    fn union_then_exclusion_is_order_independent() {
        assert_eq!(names("@all, -@all_methods, -[DEFAULT, DefaultOutput]"), ["REQUIRED", "Output"]);
        assert_eq!(names("-read, @all_methods, read"), ["edit", "take", "boxed", "new"]);
        assert_eq!(names("[new, read, new], -edit, DEFAULT"), ["read", "new", "DEFAULT"]);
        assert_eq!(
            names("[@all_ref_methods, Output], -[@all_default_methods]"),
            ["read", "Output"]
        );
    }

    #[test]
    fn empty_selection_does_not_restore_defaults() {
        assert!(names("@all, -@all").is_empty());
        assert!(names("@all_constants, -@all_constants").is_empty());
        let cfg = Config::from_tokens(quote::quote!(@all_types)).unwrap();
        let trait_ = parse_quote!(
            trait OnlyMethods {
                fn f();
            }
        );
        let selected = select(&trait_, &cfg.selection).unwrap();
        assert!(selected.methods.is_empty());
        assert!(selected.consts.is_empty());
        assert!(selected.types.is_empty());
    }

    #[test]
    fn unknown_positive_and_negative_names_are_errors() {
        for args in ["missing", "-missing", "@all, -[read, missing]", "missing, -missing"] {
            let cfg = Config::from_tokens(args.parse().unwrap()).unwrap();
            assert!(select(&fixture(), &cfg.selection).is_err(), "{args}");
        }
    }

    #[test]
    fn raw_names_resolve_without_reserving_family_names() {
        let trait_ = parse_quote!(
            trait Tr {
                fn r#type();
                fn all_methods();
            }
        );
        let cfg = Config::from_tokens(quote::quote!(r#type, all_methods)).unwrap();
        assert_eq!(select(&trait_, &cfg.selection).unwrap().methods.len(), 2);
    }
}
