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

/// Element type parameters `__T0..__T{N-1}`.
fn elem_params(n: usize) -> Vec<Ident> {
    (0..n).map(|i| format_ident!("__T{i}")).collect()
}

/// Per-element `Tr` parameters `__TA0..__TA{N-1}` (All track only).
fn tr_params(n: usize) -> Vec<Ident> {
    (0..n).map(|i| format_ident!("__TA{i}")).collect()
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
fn rewrite_where_clause(
    wc: &Option<WhereClause>, _n: usize, elems: &[Ident],
) -> Result<Option<WhereClause>> {
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
    let elems = elem_params(n);
    let tr_params_ = tr_params(n);
    let name = helper_name(trait_, n, track, has_orig_params);
    let tr = &trait_.ident;

    let mut generics = trait_.generics.clone();
    let mut params: Vec<GenericParam> = elems.iter().map(|e| parse_quote!(#e)).collect();
    match track {
        Track::Shared => params.extend(generics.params.iter().cloned()),
        Track::All if has_orig_params => params.extend(tr_params_.iter().map(|p| parse_quote!(#p))),
        Track::All => {}
    }
    generics.params = params.into_iter().collect();

    let mut predicates = rewrite_where_clause(&trait_.generics.where_clause, n, &elems)?
        .map(|wc| wc.predicates.into_iter().collect::<Vec<_>>())
        .unwrap_or_default();
    let orig_names = param_name_tokens(&trait_.generics.params);
    if has_orig_params {
        match track {
            Track::Shared => {
                for e in &elems {
                    predicates.push(parse_quote!(#e: #tr<#(#orig_names),*>));
                }
            }
            Track::All => {
                for (e, p) in elems.iter().zip(&tr_params_) {
                    predicates.push(parse_quote!(#e: #tr<#p>));
                }
            }
        }
    } else {
        for e in &elems {
            predicates.push(parse_quote!(#e: #tr));
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
    for t in &sel.types {
        items.push(TraitItem::Type(gen_helper_type(t)?));
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
    let elems = elem_params(n);
    let tr_params_ = tr_params(n);
    let name = helper_name(trait_, n, track, has_orig_params);
    let tr = &trait_.ident;

    let orig_params = trait_.generics.params.iter().cloned().collect::<Vec<_>>();
    let orig_names = param_name_tokens(&trait_.generics.params);

    let mut impl_params: Vec<GenericParam> = if has_orig_params {
        match track {
            Track::Shared => {
                let mut v = orig_params.clone();
                for e in &elems {
                    v.push(parse_quote!(#e: #tr<#(#orig_names),*>));
                }
                v
            }
            Track::All => {
                let mut v = Vec::new();
                for (e, p) in elems.iter().zip(&tr_params_) {
                    v.push(parse_quote!(#p));
                    v.push(parse_quote!(#e: #tr<#p>));
                }
                v
            }
        }
    } else {
        elems.iter().map(|e| parse_quote!(#e: #tr)).collect()
    };
    // Impl generic params cannot carry default values.
    for p in &mut impl_params {
        if let GenericParam::Type(t) = p {
            t.default = None;
        }
    }

    let mut predicates = rewrite_where_clause(&trait_.generics.where_clause, n, &elems)?
        .map(|wc| wc.predicates.into_iter().collect::<Vec<_>>())
        .unwrap_or_default();
    for t in &sel.types {
        predicates.extend(elem_typed_bounds(t, &elems)?);
    }

    let mut generics = if impl_params.is_empty() {
        syn::Generics::default()
    } else {
        parse_quote!(<#(#impl_params),*>)
    };
    if !predicates.is_empty() {
        generics.where_clause = Some(parse_quote!(where #(#predicates),*));
    }

    let helper_path: syn::Path = match (has_orig_params, track) {
        (false, _) => parse_quote!(#name<#(#elems),*>),
        (true, Track::Shared) if elems.is_empty() && orig_names.is_empty() => parse_quote!(#name),
        (true, Track::All) if elems.is_empty() && tr_params_.is_empty() => parse_quote!(#name),
        (true, Track::Shared) => parse_quote!(#name<#(#elems),*, #(#orig_names),*>),
        (true, Track::All) => parse_quote!(#name<#(#elems),*, #(#tr_params_),*>),
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
    let ret = match &sig.output {
        ReturnType::Default => parse_quote!(()),
        ReturnType::Type(_, ty) => (**ty).clone(),
    };
    sig.output = ReturnType::Type(parse_quote!(->), Box::new(tupleize(&ret, n, elems)));

    for (idx, arg) in sig.inputs.iter_mut().enumerate() {
        if let FnArg::Typed(pt) = arg {
            *pt.ty = rewrite::rewrite_param(&pt.ty, n, elems)?;
            match &*pt.pat {
                Pat::Ident(_) => {}
                Pat::Wild(_) => {
                    // `_: T` has no name to forward; give it a synthetic one.
                    let name = format_ident!("__arg{idx}");
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

    sig.generics.where_clause = rewrite_where_clause(&sig.generics.where_clause, n, elems)?;
    Ok(())
}

fn gen_helper_method(item: &TraitItemFn, n: usize, elems: &[Ident]) -> Result<TraitItemFn> {
    let mut m = item.clone();
    rewrite_signature(&mut m.sig, n, elems)?;
    m.default = None;
    Ok(m)
}

fn gen_impl_method(item: &TraitItemFn, n: usize, elems: &[Ident]) -> Result<ImplItemFn> {
    let mut m = item.clone();
    rewrite_signature(&mut m.sig, n, elems)?;
    let block = build_body(&m.sig, n, elems)?;
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
    let calls = (0..n)
        .map(|i| {
            let elem = &elems[i];
            let idx = syn::Index::from(i);
            let args = sig.inputs.iter().filter_map(|arg| match arg {
                FnArg::Typed(pt) => match &*pt.pat {
                    Pat::Ident(pi) => Some(rewrite::unpack_arg(&pt.ty, &pi.ident, i)),
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

fn gen_helper_type(item: &TraitItemType) -> Result<TraitItemType> {
    if !item.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &item.generics,
            "generic associated types are not supported",
        ));
    }
    if rewrite::bounds_contain_self(&item.bounds) {
        return Err(syn::Error::new_spanned(
            &item.bounds,
            "`Self` in an associated type bound is not supported",
        ));
    }
    let mut t = item.clone();
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
fn elem_typed_bounds(item: &TraitItemType, elems: &[Ident]) -> Result<Vec<WherePredicate>> {
    let name = &item.ident;
    let mut out = Vec::new();
    for b in &item.bounds {
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
