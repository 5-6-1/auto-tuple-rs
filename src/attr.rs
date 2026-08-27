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
//! trait items to process. Without an explicit item list all methods are
//! processed (assoc consts/types only when explicitly named).

use std::collections::BTreeSet;
use std::ops::RangeInclusive;

use proc_macro2::TokenStream;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitInt, Result, Token};

/// Parsed `#[auto_tuple(...)]` configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Inclusive range of tuple arities to generate. Defaults to `2..=12`.
    pub sizes: RangeInclusive<usize>,
    /// Explicitly selected trait items; `None` means "all supported items".
    pub items: Option<BTreeSet<String>>,
}

impl Default for Config {
    fn default() -> Self {
        Self { sizes: 2..=12, items: None }
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
        let mut items = BTreeSet::new();
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
                    return Err(input.error("expected `..` or `..=` after the range start"));
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
            } else if input.peek(Ident) {
                items.insert(input.parse::<Ident>()?.to_string());
            } else {
                return Err(input.error("expected an identifier or a range like `2..=12`"));
            }

            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }

        cfg.items = (!items.is_empty()).then_some(items);
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
        assert_eq!(cfg, Config::default());
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
        assert_eq!(cfg.items, Some(BTreeSet::from(["foo".into(), "bar".into(), "MAX".into()])));
    }

    #[test]
    fn items_only_keeps_default_range() {
        let cfg = parse("foo, bar");
        assert_eq!(cfg.sizes, 2..=12);
        assert_eq!(cfg.items, Some(BTreeSet::from(["foo".into(), "bar".into()])));
    }

    #[test]
    fn range_only_keeps_items_none() {
        assert_eq!(parse("2..=3").items, None);
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
}
