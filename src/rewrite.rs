//! Type rewriting for element-wise tuple-ization.
//!
//! Two directions are handled separately:
//! - **Return types** are rewritten element-wise: `Self` (at any nesting depth)
//!   becomes the corresponding element parameter, then the whole type is
//!   wrapped in an N-tuple. `&Self` -> `(&A, &B)`, `Box<Self>` -> `(Box<A>, Box<B>)`.
//! - **Parameter types** keep the "whole tuple" shape: `&Self` stays `&(A, B)`
//!   and is unpacked per element at call time. `Self` nested inside a generic
//!   container (`Box<Self>`, `Vec<Self>`) is unsupported in parameters.

use proc_macro2::TokenStream;
use quote::quote;
use syn::visit::Visit;
use syn::visit_mut::VisitMut;
use syn::{Ident, Result, Type, parse_quote};

/// Rewrites every `Self` occurrence in `ty` to the given element parameter.
pub fn elem_of(ty: &Type, elem: &Ident) -> Type {
    let mut ty = ty.clone();
    ElemRewriter { elem }.visit_type_mut(&mut ty);
    ty
}

/// The N-tuple form of `ty`: `(elem0(ty), ..., elem{N-1}(ty))`.
pub fn tupleize(ty: &Type, n: usize, elems: &[Ident]) -> Type {
    if n == 0 {
        return parse_quote!(());
    }
    let parts = elems.iter().map(|e| elem_of(ty, e)).collect::<Vec<_>>();
    match n {
        1 => parse_quote!((#(#parts),*,)),
        _ => parse_quote!((#(#parts),*)),
    }
}

/// Top-level `Self` shape of a parameter type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamShape {
    /// `Self`, `&Self` or `&mut Self` — kept as-is (the helper trait's own `Self`).
    SelfRef,
    /// `Self::Assoc` or `&Self::Assoc` — expanded to the element tuple.
    Assoc,
    /// No `Self` involvement — passed through unchanged.
    Plain,
}

/// Classifies a parameter type by its top-level `Self` shape.
pub fn param_shape(ty: &Type) -> Result<ParamShape> {
    match ty {
        Type::Path(tp) if tp.qself.is_none() && tp.path.is_ident("Self") => Ok(ParamShape::SelfRef),
        Type::Path(tp) if is_self_assoc(tp) => Ok(ParamShape::Assoc),
        Type::Reference(r) => {
            if is_self_type(&r.elem) {
                Ok(ParamShape::SelfRef)
            } else if matches!(&*r.elem, Type::Path(tp) if is_self_assoc(tp)) {
                Ok(ParamShape::Assoc)
            } else if contains_self(&r.elem) {
                Err(syn::Error::new_spanned(
                    r,
                    "`Self` nested inside a generic type is not supported in parameters",
                ))
            } else {
                Ok(ParamShape::Plain)
            }
        }
        other => {
            if contains_self(other) {
                Err(syn::Error::new_spanned(
                    other,
                    "`Self` nested inside a generic type is not supported in parameters",
                ))
            } else {
                Ok(ParamShape::Plain)
            }
        }
    }
}

/// The helper-trait parameter type for `ty`.
pub fn rewrite_param(ty: &Type, n: usize, elems: &[Ident]) -> Result<Type> {
    match param_shape(ty)? {
        ParamShape::SelfRef | ParamShape::Plain => Ok(ty.clone()),
        ParamShape::Assoc => match ty {
            Type::Path(_) => Ok(tupleize(ty, n, elems)),
            Type::Reference(r) => Ok(Type::Reference(syn::TypeReference {
                attrs: Vec::new(),
                and_token: r.and_token,
                lifetime: r.lifetime.clone(),
                mutability: r.mutability,
                elem: Box::new(tupleize(&r.elem, n, elems)),
            })),
            _ => unreachable!("Assoc shape implies a path or reference"),
        },
    }
}

/// Expression forwarding parameter `ident` to the `i`-th element.
///
/// Plain parameters are passed through unchanged; `Self`-shaped parameters are
/// unpacked (`x.0` / `&x.0` / `&mut x.0`), which is legal because tuple fields
/// are independent places.
pub fn unpack_arg(ty: &Type, ident: &Ident, i: usize) -> TokenStream {
    let idx = syn::Index::from(i);
    match ty {
        Type::Reference(r)
            if is_self_type(&r.elem) || matches!(&*r.elem, Type::Path(tp) if is_self_assoc(tp)) =>
        {
            let m = r.mutability;
            quote!(&#m #ident.#idx)
        }
        Type::Path(tp) if is_self_type(ty) || is_self_assoc(tp) => quote!(#ident.#idx),
        _ => quote!(#ident),
    }
}

pub(crate) fn is_self_type(ty: &Type) -> bool {
    matches!(ty, Type::Path(tp) if tp.qself.is_none() && tp.path.is_ident("Self"))
}

/// True for `Self::Assoc...` paths in both forms: `<Self as Tr>::Assoc` and
/// plain `Self::Assoc` (which syn parses as a plain multi-segment path).
fn is_self_assoc(tp: &syn::TypePath) -> bool {
    match &tp.qself {
        Some(q) => is_self_type(q.ty.as_ref()),
        None => path_starts_with_self(&tp.path),
    }
}

fn path_starts_with_self(path: &syn::Path) -> bool {
    path.segments.len() > 1 && path.segments.first().is_some_and(|s| s.ident == "Self")
}

/// Whether `Self` occurs anywhere inside `bounds` (e.g. `Foo<Self>`).
pub(crate) fn bounds_contain_self(
    bounds: &syn::punctuated::Punctuated<syn::TypeParamBound, syn::token::Plus>,
) -> bool {
    let mut finder = FindSelf(false);
    for bound in bounds {
        finder.visit_type_param_bound(bound);
    }
    finder.0
}

fn contains_self(ty: &Type) -> bool {
    let mut finder = FindSelf(false);
    finder.visit_type(ty);
    finder.0
}

/// Whether `Self` occurs anywhere in `ty` (used for where-clause checks).
pub(crate) fn type_contains_self(ty: &Type) -> bool {
    contains_self(ty)
}

struct FindSelf(bool);

impl<'ast> Visit<'ast> for FindSelf {
    fn visit_type_path(&mut self, node: &'ast syn::TypePath) {
        let hits_self = match &node.qself {
            Some(q) => is_self_type(q.ty.as_ref()),
            None => node.path.is_ident("Self") || path_starts_with_self(&node.path),
        };
        if hits_self {
            self.0 = true;
        }
        syn::visit::visit_type_path(self, node);
    }
}

struct ElemRewriter<'a> {
    elem: &'a Ident,
}

impl VisitMut for ElemRewriter<'_> {
    fn visit_type_mut(&mut self, node: &mut Type) {
        if let Type::Path(tp) = node {
            if let Some(q) = &mut tp.qself {
                if is_self_type(q.ty.as_ref()) {
                    if q.position == 1 {
                        // `Self::Assoc...` -> `elem::Assoc...`
                        let path = tp.path.clone();
                        let elem = self.elem.clone();
                        *node = parse_quote!(#elem #path);
                    } else {
                        // `<Self as Tr>::Assoc` -> `<elem as Tr>::Assoc`
                        let elem = self.elem.clone();
                        *q.ty = parse_quote!(#elem);
                    }
                    return;
                }
            } else if tp.path.is_ident("Self") {
                let elem = self.elem.clone();
                *node = parse_quote!(#elem);
                return;
            } else if path_starts_with_self(&tp.path) {
                // Plain `Self::Assoc...` path -> `elem::Assoc...`.
                let mut path = tp.path.clone();
                path.segments[0].ident = self.elem.clone();
                *node = Type::Path(syn::TypePath { attrs: Vec::new(), qself: None, path });
                return;
            }
        }
        syn::visit_mut::visit_type_mut(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::ToTokens;
    use syn::parse_quote;

    fn elems2() -> Vec<Ident> {
        vec![parse_quote!(A), parse_quote!(B)]
    }

    fn t(ty: &str) -> Type {
        syn::parse_str(ty).unwrap()
    }

    #[test]
    fn tupleize_plain_type() {
        let out = tupleize(&t("usize"), 2, &elems2());
        assert_eq!(out.to_token_stream().to_string(), "(usize , usize)");
    }

    #[test]
    fn tupleize_self() {
        let out = tupleize(&t("Self"), 2, &elems2());
        assert_eq!(out.to_token_stream().to_string(), "(A , B)");
    }

    #[test]
    fn tupleize_box_self() {
        let out = tupleize(&t("Box<Self>"), 2, &elems2());
        assert_eq!(out.to_token_stream().to_string(), "(Box < A > , Box < B >)");
    }

    #[test]
    fn tupleize_self_ref() {
        let out = tupleize(&t("&Self"), 2, &elems2());
        assert_eq!(out.to_token_stream().to_string(), "(& A , & B)");
    }

    #[test]
    fn tupleize_self_assoc() {
        let out = tupleize(&t("Self::Output"), 2, &elems2());
        assert_eq!(out.to_token_stream().to_string(), "(A :: Output , B :: Output)");
    }

    #[test]
    fn tupleize_nested_container() {
        let out = tupleize(&t("Vec<Box<Self>>"), 2, &elems2());
        assert_eq!(out.to_token_stream().to_string(), "(Vec < Box < A > > , Vec < Box < B > >)");
    }

    #[test]
    fn tupleize_unit_return() {
        let out = tupleize(&t("()"), 2, &elems2());
        assert_eq!(out.to_token_stream().to_string(), "(() , ())");
    }

    #[test]
    fn tupleize_arity_one_adds_comma() {
        let out = tupleize(&t("Self"), 1, &[parse_quote!(A)]);
        assert_eq!(out.to_token_stream().to_string(), "(A ,)");
    }

    #[test]
    fn tupleize_arity_zero_is_unit() {
        let out = tupleize(&t("Box<Self>"), 0, &[]);
        assert_eq!(out.to_token_stream().to_string(), "()");
    }

    #[test]
    fn param_self_ref_is_kept() {
        assert_eq!(param_shape(&t("&Self")).unwrap(), ParamShape::SelfRef);
        assert_eq!(param_shape(&t("&mut Self")).unwrap(), ParamShape::SelfRef);
        assert_eq!(param_shape(&t("Self")).unwrap(), ParamShape::SelfRef);
    }

    #[test]
    fn param_assoc_shape() {
        assert_eq!(param_shape(&t("Self::Output")).unwrap(), ParamShape::Assoc);
        assert_eq!(param_shape(&t("&Self::Output")).unwrap(), ParamShape::Assoc);
    }

    #[test]
    fn param_nested_self_is_rejected() {
        assert!(param_shape(&t("Box<Self>")).is_err());
        assert!(param_shape(&t("Vec<Self>")).is_err());
        assert!(param_shape(&t("&Vec<Self>")).is_err());
    }

    #[test]
    fn rewrite_param_assoc_expands() {
        let out = rewrite_param(&t("Self::Output"), 2, &elems2()).unwrap();
        assert_eq!(out.to_token_stream().to_string(), "(A :: Output , B :: Output)");
        let out = rewrite_param(&t("&Self::Output"), 2, &elems2()).unwrap();
        assert_eq!(out.to_token_stream().to_string(), "& (A :: Output , B :: Output)");
    }

    #[test]
    fn unpack_plain_and_self() {
        let x: Ident = parse_quote!(x);
        assert_eq!(unpack_arg(&t("usize"), &x, 0).to_string(), "x");
        assert_eq!(unpack_arg(&t("Self"), &x, 0).to_string(), "x . 0");
        assert_eq!(unpack_arg(&t("&Self"), &x, 1).to_string(), "& x . 1");
        assert_eq!(unpack_arg(&t("&mut Self"), &x, 0).to_string(), "& mut x . 0");
        assert_eq!(unpack_arg(&t("Self::Output"), &x, 1).to_string(), "x . 1");
        assert_eq!(unpack_arg(&t("&Self::Output"), &x, 0).to_string(), "& x . 0");
    }
}
