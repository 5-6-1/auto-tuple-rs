//! Generation of helper traits and blanket impls.

use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::parse_quote;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{
    Block, FnArg, GenericParam, Ident, ImplItem, ImplItemConst, ImplItemFn, ImplItemType, ItemImpl,
    ItemTrait, Pat, Result, ReturnType, Safety, Signature, TraitItem, TraitItemConst, TraitItemFn,
    TraitItemType, Type, WhereClause, WherePredicate,
};

use crate::analyze::Track;
use crate::rewrite::{self, tupleize};

/// Names of the trait's generic parameters (for bound-dropping decisions).
fn trait_param_names(generics: &syn::Generics) -> std::collections::BTreeSet<String> {
    generics
        .params
        .iter()
        .map(|p| match p {
            GenericParam::Type(t) => t.ident.to_string(),
            GenericParam::Lifetime(l) => l.lifetime.ident.to_string(),
            GenericParam::Const(c) => c.ident.to_string(),
        })
        .collect()
}

/// Whether a bound should be kept (element-wise) on the helper trait.
///
/// The helper trait's associated value is the element tuple, so a bound is
/// kept only if the tuple provably satisfies it: whitelisted arg-less traits
/// (`Clone`, `Copy`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash`, `Debug`,
/// `Default`, `Sized`) or lifetimes. Everything else — traits with args
/// (`AsRef<T>`), non-whitelisted traits (`Iterator`, `Add`, ...), and any
/// bound mentioning `Self` or an original trait generic param — is dropped:
/// the element side is already guaranteed by `A: Tr<...>`.
fn keep_elem_bound(
    bound: &syn::TypeParamBound, trait_params: &std::collections::BTreeSet<String>,
) -> bool {
    if bound_mentions_self_or_params(bound, trait_params) {
        return false;
    }
    match bound {
        syn::TypeParamBound::Lifetime(_) => true,
        syn::TypeParamBound::Trait(tb) => {
            let Some(seg) = tb.path.segments.last() else {
                return false;
            };
            let whitelisted = matches!(
                seg.ident.to_string().as_str(),
                "Clone"
                    | "Copy"
                    | "PartialEq"
                    | "Eq"
                    | "PartialOrd"
                    | "Ord"
                    | "Hash"
                    | "Debug"
                    | "Default"
                    | "Sized"
            );
            whitelisted && matches!(seg.arguments, syn::PathArguments::None)
        }
        _ => false,
    }
}

/// Whether a bound mentions `Self` or an original trait generic parameter.
///
/// Such bounds cannot be kept on the helper trait: its associated value is the
/// element tuple, which does not satisfy them. The element side is already
/// guaranteed by `A: Tr<...>`.
fn bound_mentions_self_or_params(
    bound: &syn::TypeParamBound, trait_params: &std::collections::BTreeSet<String>,
) -> bool {
    use syn::visit::Visit;
    struct Find<'a> {
        params: &'a std::collections::BTreeSet<String>,
        hit: bool,
    }
    impl<'ast> Visit<'ast> for Find<'_> {
        fn visit_type_path(&mut self, node: &'ast syn::TypePath) {
            let self_hit = match &node.qself {
                Some(q) => rewrite::is_self_type(q.ty.as_ref()),
                None => {
                    node.path.is_ident("Self")
                        || node.path.segments.first().is_some_and(|s| s.ident == "Self")
                }
            };
            if self_hit {
                self.hit = true;
            }
            syn::visit::visit_type_path(self, node);
        }
        fn visit_path(&mut self, node: &'ast syn::Path) {
            if node.segments.first().is_some_and(|seg| self.params.contains(&seg.ident.to_string()))
            {
                self.hit = true;
            }
            syn::visit::visit_path(self, node);
        }
    }
    let mut finder = Find { params: trait_params, hit: false };
    finder.visit_type_param_bound(bound);
    finder.hit
}

/// Whether a type contains a `use<..>` precise-capturing bound.
fn contains_precise_capture(ty: &Type) -> bool {
    use syn::visit::Visit;
    struct Find(bool);
    impl<'ast> syn::visit::Visit<'ast> for Find {
        fn visit_type_param_bound(&mut self, node: &'ast syn::TypeParamBound) {
            if matches!(node, syn::TypeParamBound::PreciseCapture(_)) {
                self.0 = true;
            }
            syn::visit::visit_type_param_bound(self, node);
        }
    }
    let mut finder = Find(false);
    finder.visit_type(ty);
    finder.0
}

/// The trait items selected for tuple-ization.
pub struct Selected<'a> {
    pub methods: Vec<&'a TraitItemFn>,
    pub consts: Vec<&'a TraitItemConst>,
    pub types: Vec<&'a TraitItemType>,
}

/// Generates the helper trait and blanket impl for one arity.
pub fn generate(
    trait_: &ItemTrait, track: Track, selected: &Selected, n: usize,
) -> Result<TokenStream> {
    let helper = build_helper_trait(trait_, track, selected, n)?;
    let impl_ = build_impl(trait_, track, selected, n)?;
    Ok(quote!(#helper #impl_))
}

/// Element type parameters `__T0..__T{N-1}`, avoiding clashes with the
/// trait's own generic parameter names.
fn elem_params(n: usize, taken: &std::collections::BTreeSet<String>) -> Vec<Ident> {
    (0..n).map(|i| fresh_param_name(format!("__T{i}"), taken)).collect()
}

/// All-track per-element `Tr` parameter groups, mirroring the original trait
/// parameter shape: one group per element, one parameter per original
/// parameter (`__T{i}_{name}` type params with bounds carried over,
/// `'__L{i}_{name}` lifetimes, `const __C{i}_{name}: Ty` const params).
///
/// Returns the generic params to declare (helper trait and impl) and, per
/// element, the name tokens used in bounds and instantiations.
fn all_track_params(
    trait_: &ItemTrait, n: usize, taken: &std::collections::BTreeSet<String>,
) -> (Vec<GenericParam>, Vec<Vec<TokenStream>>) {
    let mut params = Vec::new();
    let mut per_elem = Vec::new();
    for i in 0..n {
        let mut names = Vec::new();
        for orig in &trait_.generics.params {
            match orig {
                GenericParam::Type(t) => {
                    let name = fresh_param_name(format!("__T{i}_{}", t.ident), taken);
                    let mut tp: GenericParam = parse_quote!(#name);
                    if let GenericParam::Type(tp) = &mut tp {
                        // Carry only whitelisted bounds (the element tuple
                        // provably satisfies them); anything mentioning `Self`
                        // or an original param was already routed to Shared.
                        tp.bounds = t
                            .bounds
                            .iter()
                            .filter(|b| keep_elem_bound(b, taken))
                            .cloned()
                            .collect();
                    }
                    params.push(tp);
                    names.push(quote!(#name));
                }
                GenericParam::Lifetime(l) => {
                    let name = fresh_param_name(format!("__L{i}_{}", l.lifetime.ident), taken);
                    let lt =
                        syn::Lifetime::new(&format!("'{name}"), proc_macro2::Span::call_site());
                    params.push(GenericParam::Lifetime(syn::LifetimeParam {
                        attrs: Vec::new(),
                        lifetime: lt.clone(),
                        colon_token: None,
                        bounds: Default::default(),
                    }));
                    names.push(quote!(#lt));
                }
                GenericParam::Const(c) => {
                    let name = fresh_param_name(format!("__C{i}_{}", c.ident), taken);
                    let ty = &c.ty;
                    params.push(parse_quote!(const #name: #ty));
                    names.push(quote!(#name));
                }
            }
        }
        per_elem.push(names);
    }
    (params, per_elem)
}

/// A generated parameter name that collides with nothing in `taken`; a
/// numeric suffix keeps the search bounded (unlike appending underscores).
fn fresh_param_name(base: String, taken: &std::collections::BTreeSet<String>) -> Ident {
    if !taken.contains(&base) {
        return format_ident!("{}", base);
    }
    let mut i = 1;
    loop {
        let name = format!("{base}_{i}");
        if !taken.contains(&name) {
            return format_ident!("{}", name);
        }
        i += 1;
    }
}

/// Synthetic name for an unnamed (`_`) parameter that collides with nothing.
///
/// Derived from the *original* parameter names only, so the helper-side
/// signature and the impl-side body agree on the same name.
fn synthetic_arg_name(taken: &std::collections::BTreeSet<String>, pidx: usize) -> Ident {
    let mut name = format_ident!("__arg{pidx}");
    while taken.contains(&name.to_string()) {
        name = format_ident!("{}_", name);
    }
    name
}

/// Helper trait name: `_TrTuple2` / `_TrTuple2All`.
///
/// The `All` suffix only applies when the original trait has generic
/// parameters (so per-element parameters are meaningful); a trait without
/// generic parameters always uses the plain name.
fn helper_name(trait_: &ItemTrait, n: usize, track: Track, has_orig_params: bool) -> Ident {
    let suffix = match (track, has_orig_params) {
        (Track::All, true) => "All",
        _ => "",
    };
    format_ident!("_{}Tuple{}{}", trait_.ident, n, suffix)
}

/// Tokens naming each original generic parameter (`T`, `'a`, `N`).
fn param_name_tokens(params: &Punctuated<GenericParam, Comma>) -> Vec<TokenStream> {
    params
        .iter()
        .map(|p| match p {
            GenericParam::Type(t) => t.ident.to_token_stream(),
            GenericParam::Lifetime(l) => l.lifetime.to_token_stream(),
            GenericParam::Const(c) => c.ident.to_token_stream(),
        })
        .collect()
}

/// Tuple type `(__T0, __T1)`, `(__T0,)` or `()`.
fn tuple_type(n: usize, elems: &[Ident]) -> Type {
    match n {
        0 => parse_quote!(()),
        1 => parse_quote!((#(#elems),*,)),
        _ => parse_quote!((#(#elems),*)),
    }
}

/// Rewrites a where clause for the helper side: `Self: Foo` splits into one
/// predicate per element; `Self` in any other position is rejected.
fn rewrite_where_clause(wc: &Option<WhereClause>, elems: &[Ident]) -> Result<Option<WhereClause>> {
    let Some(wc) = wc else {
        return Ok(None);
    };
    let mut out: Vec<WherePredicate> = Vec::new();
    for p in &wc.predicates {
        match p {
            WherePredicate::Type(pt) if rewrite::is_self_type(&pt.bounded_ty) => {
                if rewrite::bounds_contain_self(&pt.bounds) {
                    return Err(syn::Error::new_spanned(
                        &pt.bounds,
                        "`Self` inside a where-clause bound is not supported",
                    ));
                }
                for e in elems {
                    let bounds = &pt.bounds;
                    out.push(parse_quote!(#e: #bounds));
                }
            }
            WherePredicate::Type(pt) if matches!(&pt.bounded_ty, Type::Path(tp) if rewrite::is_self_assoc(tp)) =>
            {
                // `where Self::Output: Clone` -> per-element projections
                // `A::Output: Clone, B::Output: Clone`. The subject is
                // rewritten by replacing the leading `Self` segment (same rule
                // as return types), so multi-level projections keep their full
                // path: `Self::Output::Item` -> `A::Output::Item`.
                for e in elems {
                    let subject: Type = rewrite::elem_of(&pt.bounded_ty, e);
                    let bounds = &pt.bounds;
                    out.push(parse_quote!(#subject: #bounds));
                }
                // The projection subject still mentions `Self`, so it must not
                // fall through to the generic `Self`-rejecting arm below; this
                // `continue` is required, not dead code.
                continue;
            }
            WherePredicate::Type(pt) => {
                if rewrite::type_contains_self(&pt.bounded_ty)
                    || rewrite::bounds_contain_self(&pt.bounds)
                {
                    return Err(syn::Error::new_spanned(
                        p,
                        "`Self` inside a where clause is only supported as the predicate subject",
                    ));
                }
                out.push(p.clone());
            }
            _ => out.push(p.clone()),
        }
    }
    Ok(Some(parse_quote!(where #(#out),*)))
}

fn build_helper_trait(
    trait_: &ItemTrait, track: Track, sel: &Selected, n: usize,
) -> Result<ItemTrait> {
    let has_orig_params = !trait_.generics.params.is_empty();
    let taken = trait_param_names(&trait_.generics);
    let elems = elem_params(n, &taken);
    let name = helper_name(trait_, n, track, has_orig_params);
    let tr = &trait_.ident;

    let mut generics = trait_.generics.clone();
    let mut params: Vec<GenericParam> = elems.iter().map(|e| parse_quote!(#e)).collect();
    let all_per_elem: Vec<Vec<TokenStream>> = match track {
        Track::Shared => {
            params.extend(generics.params.iter().cloned());
            Vec::new()
        }
        Track::All if has_orig_params => {
            let (mut group, per_elem) = all_track_params(trait_, n, &taken);
            params.append(&mut group);
            per_elem
        }
        Track::All => Vec::new(),
    };
    generics.params = params.into_iter().collect();

    let mut predicates = rewrite_where_clause(&trait_.generics.where_clause, &elems)?
        .map(|wc| wc.predicates.into_iter().collect::<Vec<_>>())
        .unwrap_or_default();
    let orig_names = param_name_tokens(&trait_.generics.params);
    match track {
        Track::Shared => {
            for e in &elems {
                predicates.push(parse_quote!(#e: #tr<#(#orig_names),*>));
            }
        }
        Track::All if has_orig_params => {
            for (i, e) in elems.iter().enumerate() {
                let names = &all_per_elem[i];
                predicates.push(parse_quote!(#e: #tr<#(#names),*>));
            }
        }
        _ => {
            for e in &elems {
                predicates.push(parse_quote!(#e: #tr));
            }
        }
    }
    if !predicates.is_empty() {
        generics.where_clause = Some(parse_quote!(where #(#predicates),*));
    }

    let mut items: Vec<TraitItem> = Vec::new();
    for m in &sel.methods {
        items.push(TraitItem::Fn(gen_helper_method(m, n, &elems)?));
    }
    for c in &sel.consts {
        items.push(TraitItem::Const(gen_helper_const(c, n, &elems)?));
    }
    let trait_params = trait_param_names(&trait_.generics);
    for t in &sel.types {
        items.push(TraitItem::Type(gen_helper_type(t, &trait_params)?));
    }

    Ok(ItemTrait {
        attrs: vec![parse_quote!(#[doc(hidden)])],
        vis: trait_.vis.clone(),
        modifiers: syn::TraitModifiers::default(),
        unsafety: trait_.unsafety,
        trait_token: Default::default(),
        ident: name,
        generics,
        colon_token: None,
        supertraits: Default::default(),
        brace_token: Default::default(),
        items: items.into_iter().collect(),
    })
}

fn build_impl(trait_: &ItemTrait, track: Track, sel: &Selected, n: usize) -> Result<ItemImpl> {
    let has_orig_params = !trait_.generics.params.is_empty();
    let taken = trait_param_names(&trait_.generics);
    let elems = elem_params(n, &taken);
    let name = helper_name(trait_, n, track, has_orig_params);
    let tr = &trait_.ident;

    let orig_params = trait_.generics.params.iter().cloned().collect::<Vec<_>>();
    let orig_names = param_name_tokens(&trait_.generics.params);

    let (mut impl_params, all_per_elem): (Vec<GenericParam>, Vec<Vec<TokenStream>>) =
        if has_orig_params {
            match track {
                Track::Shared => (
                    {
                        let mut v = orig_params.clone();
                        for e in &elems {
                            v.push(parse_quote!(#e: #tr<#(#orig_names),*>));
                        }
                        v
                    },
                    Vec::new(),
                ),
                Track::All => {
                    let (mut group, per_elem) = all_track_params(trait_, n, &taken);
                    let mut v = Vec::new();
                    v.append(&mut group);
                    for (i, e) in elems.iter().enumerate() {
                        let names = &per_elem[i];
                        v.push(parse_quote!(#e: #tr<#(#names),*>));
                    }
                    (v, per_elem)
                }
            }
        } else {
            (elems.iter().map(|e| parse_quote!(#e: #tr)).collect(), Vec::new())
        };
    // Impl generic params cannot carry default values.
    for p in &mut impl_params {
        if let GenericParam::Type(t) = p {
            t.default = None;
        }
    }

    let mut predicates = rewrite_where_clause(&trait_.generics.where_clause, &elems)?
        .map(|wc| wc.predicates.into_iter().collect::<Vec<_>>())
        .unwrap_or_default();
    let trait_params = trait_param_names(&trait_.generics);
    for t in &sel.types {
        predicates.extend(elem_typed_bounds(t, &elems, &trait_params)?);
    }

    let mut generics = if impl_params.is_empty() {
        syn::Generics::default()
    } else {
        parse_quote!(<#(#impl_params),*>)
    };
    if !predicates.is_empty() {
        generics.where_clause = Some(parse_quote!(where #(#predicates),*));
    }

    let all_names: Vec<TokenStream> = all_per_elem.iter().flatten().cloned().collect();
    let helper_path: syn::Path = match (has_orig_params, track) {
        (false, _) => parse_quote!(#name<#(#elems),*>),
        (true, Track::Shared) if elems.is_empty() && orig_names.is_empty() => parse_quote!(#name),
        (true, Track::All) if elems.is_empty() && all_names.is_empty() => parse_quote!(#name),
        (true, Track::Shared) => parse_quote!(#name<#(#elems),*, #(#orig_names),*>),
        (true, Track::All) => parse_quote!(#name<#(#elems),*, #(#all_names),*>),
    };
    let self_ty = tuple_type(n, &elems);

    let mut items: Vec<ImplItem> = Vec::new();
    for m in &sel.methods {
        items.push(ImplItem::Fn(gen_impl_method(m, n, &elems)?));
    }
    for c in &sel.consts {
        items.push(ImplItem::Const(gen_impl_const(c, n, &elems)?));
    }
    for t in &sel.types {
        items.push(ImplItem::Type(gen_impl_type(t, n, &elems)?));
    }

    Ok(ItemImpl {
        attrs: Vec::new(),
        modifiers: syn::ImplModifiers::default(),
        unsafety: trait_.unsafety,
        impl_token: Default::default(),
        generics,
        trait_: Some((helper_path, parse_quote!(for))),
        self_ty: Box::new(self_ty),
        brace_token: Default::default(),
        items: items.into_iter().collect(),
    })
}

/// Rewrites a method signature for the helper side (return, params, where).
fn rewrite_signature(sig: &mut Signature, n: usize, elems: &[Ident]) -> Result<()> {
    if let Some(r) = sig.receiver().filter(|r| matches!(r.kind, syn::ReceiverKind::Typed(..))) {
        return Err(syn::Error::new_spanned(
            r,
            "custom receivers (`self: Type`) are not supported",
        ));
    }
    let ret = match &sig.output {
        ReturnType::Default => parse_quote!(()),
        ReturnType::Type(_, ty) => (**ty).clone(),
    };
    if contains_precise_capture(&ret) {
        return Err(syn::Error::new_spanned(
            &ret,
            "`use<..>` precise capturing in returns is not supported yet",
        ));
    }
    sig.output = ReturnType::Type(parse_quote!(->), Box::new(tupleize(&ret, n, elems)));

    let orig_arg_names: std::collections::BTreeSet<String> = sig
        .inputs
        .iter()
        .filter_map(|a| match a {
            FnArg::Typed(pt) => match &*pt.pat {
                Pat::Ident(pi) => Some(pi.ident.to_string()),
                _ => None,
            },
            _ => None,
        })
        .collect();
    for (idx, arg) in sig.inputs.iter_mut().enumerate() {
        if let FnArg::Typed(pt) = arg {
            *pt.ty = rewrite::rewrite_param(&pt.ty, n, elems)?;
            match &*pt.pat {
                Pat::Ident(_) => {}
                Pat::Wild(_) => {
                    // `_: T` has no name to forward; give it a synthetic one
                    // that collides with no existing parameter name.
                    let name = synthetic_arg_name(&orig_arg_names, idx);
                    *pt.pat = parse_quote!(#name);
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        &*pt.pat,
                        "unsupported parameter pattern; use a plain identifier",
                    ));
                }
            }
        }
    }

    sig.generics.where_clause = rewrite_where_clause(&sig.generics.where_clause, elems)?;
    Ok(())
}

fn gen_helper_method(item: &TraitItemFn, n: usize, elems: &[Ident]) -> Result<TraitItemFn> {
    let mut m = item.clone();
    rewrite_signature(&mut m.sig, n, elems)?;
    m.default = None;
    Ok(m)
}

fn gen_impl_method(item: &TraitItemFn, n: usize, elems: &[Ident]) -> Result<ImplItemFn> {
    // Build the body from the *original* signature: unpacking decisions must
    // see the pre-rewrite parameter types (`Self::Output`, `&Self`, ...).
    let block = build_body(&item.sig, n, elems)?;
    let mut m = item.clone();
    rewrite_signature(&mut m.sig, n, elems)?;
    Ok(ImplItemFn {
        attrs: Vec::new(),
        vis: syn::Visibility::Inherited,
        modifiers: syn::FnModifiers::default(),
        sig: m.sig,
        block,
    })
}

fn build_body(sig: &Signature, n: usize, elems: &[Ident]) -> Result<Block> {
    if n == 0 {
        return Ok(parse_quote!({}));
    }
    let name = &sig.ident;
    let orig_arg_names: std::collections::BTreeSet<String> = sig
        .inputs
        .iter()
        .filter_map(|a| match a {
            FnArg::Typed(pt) => match &*pt.pat {
                Pat::Ident(pi) => Some(pi.ident.to_string()),
                _ => None,
            },
            _ => None,
        })
        .collect();
    let calls = (0..n)
        .map(|i| {
            let elem = &elems[i];
            let idx = syn::Index::from(i);
            let args = sig.inputs.iter().enumerate().filter_map(|(pidx, arg)| match arg {
                FnArg::Typed(pt) => match &*pt.pat {
                    Pat::Ident(pi) => Some(rewrite::unpack_arg(&pt.ty, &pi.ident, i)),
                    // `_: T` gets the same synthetic name as in rewrite_signature.
                    Pat::Wild(_) => {
                        let name = synthetic_arg_name(&orig_arg_names, pidx);
                        Some(rewrite::unpack_arg(&pt.ty, &name, i))
                    }
                    _ => None,
                },
                FnArg::Receiver(_) => None,
            });
            let mut call = match sig.receiver() {
                Some(_) => quote!(self.#idx.#name(#(#args),*)),
                None => quote!(#elem::#name(#(#args),*)),
            };
            if sig.asyncness.is_some() {
                call = quote!(#call.await);
            }
            call
        })
        .collect::<Vec<_>>();

    let tuple = match n {
        1 => quote!((#(#calls),*,)),
        _ => quote!((#(#calls),*)),
    };
    let body = if matches!(sig.safety, Safety::Unsafe(_)) {
        quote!({ unsafe { #tuple } })
    } else {
        quote!({ #tuple })
    };
    Ok(parse_quote!(#body))
}

fn gen_helper_const(item: &TraitItemConst, n: usize, elems: &[Ident]) -> Result<TraitItemConst> {
    let mut c = item.clone();
    c.ty = tupleize(&c.ty, n, elems);
    c.default = None;
    Ok(c)
}

fn gen_impl_const(item: &TraitItemConst, n: usize, elems: &[Ident]) -> Result<ImplItemConst> {
    let ty = tupleize(&item.ty, n, elems);
    let name = &item.ident;
    let value: TokenStream = match n {
        0 => quote!(()),
        1 => quote!((#(#elems::#name),*,)),
        _ => quote!((#(#elems::#name),*)),
    };
    let item: ImplItemConst = parse_quote! {
        const #name: #ty = #value;
    };
    Ok(item)
}

fn gen_helper_type(
    item: &TraitItemType, trait_params: &std::collections::BTreeSet<String>,
) -> Result<TraitItemType> {
    if !item.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &item.generics,
            "generic associated types are not supported",
        ));
    }
    let mut t = item.clone();
    // Keep only bounds the element tuple provably satisfies (whitelist); the
    // rest are dropped — the element side is guaranteed by `A: Tr<...>`.
    t.bounds = item.bounds.iter().filter(|b| keep_elem_bound(b, trait_params)).cloned().collect();
    t.default = None;
    Ok(t)
}

fn gen_impl_type(item: &TraitItemType, n: usize, elems: &[Ident]) -> Result<ImplItemType> {
    let name = &item.ident;
    let assoc: Type = parse_quote!(Self::#name);
    let ty = tupleize(&assoc, n, elems);
    let item: ImplItemType = parse_quote! {
        type #name = #ty;
    };
    Ok(item)
}

/// Element-wise copies of an associated type's bounds, e.g. `A::Output: Clone`.
///
/// Only whitelisted bounds (which the tuple provably satisfies) are copied;
/// the rest are guaranteed by `A: Tr<...>` on the element side.
fn elem_typed_bounds(
    item: &TraitItemType, elems: &[Ident], trait_params: &std::collections::BTreeSet<String>,
) -> Result<Vec<WherePredicate>> {
    let name = &item.ident;
    let mut out = Vec::new();
    for b in &item.bounds {
        if !keep_elem_bound(b, trait_params) {
            continue;
        }
        let bound = match b {
            syn::TypeParamBound::Trait(tb) => tb.to_token_stream(),
            syn::TypeParamBound::Lifetime(l) => l.to_token_stream(),
            _ => {
                return Err(syn::Error::new_spanned(b, "unsupported associated type bound"));
            }
        };
        for e in elems {
            out.push(parse_quote!(#e::#name: #bound));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn const_param_all_track_generates() {
        // Regression: `#c.ty` inside quote! used to emit a literal `.ty`
        // suffix, breaking `const __C{i}_{name}: Ty` generation.
        let trait_: ItemTrait = parse_quote! {
            trait WithConst<const N: usize> {
                fn cf(&self) -> usize;
            }
        };
        let methods = trait_
            .items
            .iter()
            .filter_map(|i| match i {
                TraitItem::Fn(f) => Some(f),
                _ => None,
            })
            .collect::<Vec<_>>();
        let sel = Selected { methods, consts: Vec::new(), types: Vec::new() };
        let out = generate(&trait_, Track::All, &sel, 2).unwrap();
        assert!(!out.to_string().is_empty());
    }

    #[test]
    fn where_projection_replaces_leading_self() {
        // `Self::Output::Item: Clone` must become `A::Output::Item: Clone`,
        // not `A::Item: Clone` (multi-level projection).
        let trait_: ItemTrait = parse_quote! {
            trait Tr {
                fn get(&self)
                where
                    Self::Output::Item: Clone;
            }
        };
        let methods = trait_
            .items
            .iter()
            .filter_map(|i| match i {
                TraitItem::Fn(f) => Some(f),
                _ => None,
            })
            .collect::<Vec<_>>();
        let sel = Selected { methods, consts: Vec::new(), types: Vec::new() };
        let out = generate(&trait_, Track::All, &sel, 2).unwrap().to_string();
        assert!(
            out.contains("__T0 :: Output :: Item : Clone"),
            "expected per-element projection, got: {out}"
        );
        assert!(
            !out.contains("__T0 :: Item : Clone"),
            "must not drop intermediate projection: {out}"
        );
    }
}
