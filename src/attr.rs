//! Parsing of the `#[auto_tuple(...)]` attribute arguments.
//!
//! Supported forms:
//! ```text
//! #[auto_tuple]
//! #[auto_tuple(2..=12)]
//! #[auto_tuple(foo, 2..=12)]
//! #[auto_tuple(foo, MAX, Output, 2..=12)]
//! ```
//!
//! A range selects the tuple arities to generate; identifiers select the
//! trait items to process. Selection defaults to all methods, associated
//! constants and associated types; families and exclusions refine that set.

use std::ops::RangeInclusive;

use proc_macro2::TokenStream;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitInt, Result, Token, Visibility};

use crate::selection::Selection;

/// Parsed `#[auto_tuple(...)]` configuration.
#[derive(Clone)]
pub struct Config {
    /// Inclusive range of tuple arities to generate. Defaults to `2..=12`.
    pub sizes: RangeInclusive<usize>,
    /// Member-set expressions, resolved before analysis and code generation.
    pub selection: Selection,
    /// Visibility override for the generated helper traits; `None` inherits
    /// the original trait's visibility.
    pub vis: Option<Visibility>,
    /// Name prefix override for the generated helper traits (`name = "X"`
    /// yields `XTuple2` / `XTuple2All`); `None` uses `_{Trait}Tuple{N}`.
    pub name: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self { sizes: 2..=12, selection: Selection::default(), vis: None, name: None }
    }
}

impl Config {
    /// Parses the attribute arguments; an empty stream yields the defaults.
    pub fn from_tokens(args: TokenStream) -> Result<Self> {
        if args.is_empty() { Ok(Self::default()) } else { syn::parse2(args) }
    }
}

impl Parse for Config {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut cfg = Self::default();
        let mut range_seen = false;

        while !input.is_empty() {
            if input.peek(LitInt) {
                if range_seen {
                    return Err(input.error("multiple ranges are not allowed"));
                }
                range_seen = true;
                let start = input.parse::<LitInt>()?.base10_parse::<usize>()?;
                let (inclusive, end) = if input.peek(Token![..=]) {
                    input.parse::<Token![..=]>()?;
                    (true, input.parse::<LitInt>()?.base10_parse::<usize>()?)
                } else if input.peek(Token![..]) {
                    input.parse::<Token![..]>()?;
                    (false, input.parse::<LitInt>()?.base10_parse::<usize>()?)
                } else {
                    return Err(
                        input.error("expected `..` or `..=` after the range start, e.g. `2..=12`")
                    );
                };
                let end = if inclusive {
                    end
                } else {
                    end.checked_sub(1).ok_or_else(|| input.error("range must be non-empty"))?
                };
                if start > end {
                    return Err(input.error("range start must not exceed its end"));
                }
                cfg.sizes = start..=end;
            } else if input.peek(Ident) && input.peek2(Token![=]) {
                // `name = "X"` option; an ident followed by `=` cannot be a
                // trait item selector.
                let key: Ident = input.parse()?;
                if key != "name" {
                    return Err(input.error("unknown option; expected `name`"));
                }
                input.parse::<Token![=]>()?;
                let lit: syn::LitStr = input.parse()?;
                if cfg.name.is_some() {
                    return Err(input.error("duplicate `name` option"));
                }
                let name = lit.value();
                // `format_ident!` panics on illegal identifiers; validate here
                // so the user gets a proper error instead.
                syn::parse_str::<Ident>(&name)
                    .map_err(|_| input.error("`name` must be a valid Rust identifier prefix"))?;
                cfg.name = Some(name);
            } else if input.peek(Ident)
                || input.peek(Token![@])
                || input.peek(Token![-])
                || input.peek(syn::token::Bracket)
            {
                cfg.selection.parse_term(input)?;
            } else if input.peek(Token![pub]) {
                let vis: Visibility = input.parse()?;
                if cfg.vis.is_some() {
                    return Err(input.error("duplicate visibility specifier"));
                }
                cfg.vis = Some(vis);
            } else {
                return Err(input
                    .error("expected a member selector, a range like `2..=12`, or a visibility"));
            }

            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }

        Ok(cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> Config {
        syn::parse_str(input).unwrap()
    }

    #[test]
    fn default_range_is_two_through_twelve() {
        assert_eq!(Config::default().sizes, 2..=12);
    }

    #[test]
    fn empty_args_use_defaults() {
        let cfg = Config::from_tokens(TokenStream::new()).unwrap();
        assert_eq!(cfg.sizes, 2..=12);
        assert!(cfg.vis.is_none());
    }

    #[test]
    fn parses_inclusive_range() {
        assert_eq!(parse("2..=12").sizes, 2..=12);
    }

    #[test]
    fn parses_exclusive_range() {
        assert_eq!(parse("0..12").sizes, 0..=11);
    }

    #[test]
    fn parses_items_and_range() {
        let cfg = parse("foo, bar, MAX, 2..=4");
        assert_eq!(cfg.sizes, 2..=4);
    }

    #[test]
    fn items_only_keeps_default_range() {
        let cfg = parse("foo, bar");
        assert_eq!(cfg.sizes, 2..=12);
    }

    #[test]
    fn parses_families_lists_and_exclusions() {
        parse("@all_methods, MAX, -read, -[reset, clear], -@all_types, [Output], 2..=3,");
    }

    #[test]
    fn rejects_inverted_range() {
        assert!(syn::parse_str::<Config>("5..=2").is_err());
    }

    #[test]
    fn rejects_empty_exclusive_range() {
        assert!(syn::parse_str::<Config>("2..2").is_err());
    }

    #[test]
    fn rejects_dangling_range_end() {
        assert!(syn::parse_str::<Config>("2..").is_err());
    }

    #[test]
    fn rejects_leading_comma() {
        assert!(syn::parse_str::<Config>(", foo").is_err());
    }

    #[test]
    fn rejects_multiple_ranges() {
        assert!(syn::parse_str::<Config>("2..=4, 6..=8").is_err());
    }

    #[test]
    fn parses_visibility_override() {
        let cfg = parse("pub(crate), foo");
        assert!(cfg.vis.is_some());
    }

    #[test]
    fn rejects_duplicate_visibility() {
        assert!(syn::parse_str::<Config>("pub, pub(crate)").is_err());
    }

    #[test]
    fn rejects_illegal_name() {
        assert!(syn::parse_str::<Config>("name = \"foo-bar\"").is_err());
    }

    #[test]
    fn rejects_duplicate_name() {
        assert!(syn::parse_str::<Config>("name = \"A\", name = \"B\"").is_err());
    }

    #[test]
    fn rejects_malformed_selectors() {
        for args in [
            "@",
            "@missing",
            "-",
            "-@missing",
            "[]",
            "-[]",
            "[foo,,bar]",
            "[[foo]]",
            "[foo, -bar]",
            "@all_methods foo",
            "@all,,foo",
            "[2..=4]",
        ] {
            assert!(syn::parse_str::<Config>(args).is_err(), "{args}");
        }
    }
}
