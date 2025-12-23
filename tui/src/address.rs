//! Addressing DSL for targeting widgets in compact trees.
//!
//! Syntax:
//! - `id` - Select node by InteractionKey ID
//! - `id[n]` - Select nth instance (0-indexed)
//! - `id.child` - Navigate to child node
//! - `id:variant[n]` - Select nth instance of variant (grouped by child structure)
//!
//! This module is tested and planned for future use in TUI navigation.
#![allow(dead_code)]

use std::fmt;

use crate::tree_format::CompactTree;

/// A parsed DSL path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Address {
    pub segments: Vec<AddressSegment>,
}

/// A single segment in the path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddressSegment {
    /// Just an ID: `item`
    Id(String),
    /// ID with index: `item[2]`
    Indexed { id: String, index: usize },
    /// ID with variant filter: `item:delete`
    VariantFilter { id: String, variant_child: String },
    /// ID with variant filter and index: `item:delete[1]`
    VariantFilterIndexed {
        id: String,
        variant_child: String,
        index: usize,
    },
}

/// Error during parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    EmptyInput,
    EmptySegment,
    EmptyId,
    EmptyVariantChild,
    InvalidIndex(String),
    UnclosedBracket,
    UnexpectedCharacter(char),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::EmptyInput => write!(f, "empty input"),
            ParseError::EmptySegment => write!(f, "empty segment"),
            ParseError::EmptyId => write!(f, "empty id"),
            ParseError::EmptyVariantChild => write!(f, "empty variant child"),
            ParseError::InvalidIndex(s) => write!(f, "invalid index: {}", s),
            ParseError::UnclosedBracket => write!(f, "unclosed bracket"),
            ParseError::UnexpectedCharacter(c) => write!(f, "unexpected character: {}", c),
        }
    }
}

impl std::error::Error for ParseError {}

/// Error during resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    NotFound(String),
    IndexOutOfBounds { id: String, index: usize, count: usize },
    VariantNotFound { id: String, variant_child: String },
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResolveError::NotFound(id) => write!(f, "node not found: {}", id),
            ResolveError::IndexOutOfBounds { id, index, count } => {
                write!(f, "index {} out of bounds for {} (count: {})", index, id, count)
            }
            ResolveError::VariantNotFound { id, variant_child } => {
                write!(f, "variant {}:{} not found", id, variant_child)
            }
        }
    }
}

impl std::error::Error for ResolveError {}

/// A resolved address ready for execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedAddress {
    pub path: Vec<ResolvedStep>,
}

/// A single step in a resolved path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedStep {
    pub id: String,
    pub instance_index: usize,
}

impl Address {
    /// Parse a DSL string into an Address.
    pub fn parse(input: &str) -> Result<Address, ParseError> {
        if input.is_empty() {
            return Err(ParseError::EmptyInput);
        }

        let mut segments = Vec::new();
        for part in input.split('.') {
            if part.is_empty() {
                return Err(ParseError::EmptySegment);
            }
            segments.push(AddressSegment::parse(part)?);
        }

        Ok(Address { segments })
    }

    /// Resolve this address against a CompactTree.
    /// Returns the resolved path indices for execution.
    #[allow(unused_variables)]
    pub fn resolve(&self, tree: &CompactTree) -> Result<ResolvedAddress, ResolveError> {
        todo!("Will implement in TUI integration")
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, segment) in self.segments.iter().enumerate() {
            if i > 0 {
                write!(f, ".")?;
            }
            write!(f, "{}", segment)?;
        }
        Ok(())
    }
}

impl AddressSegment {
    /// Parse a single segment.
    fn parse(input: &str) -> Result<AddressSegment, ParseError> {
        // Check for variant filter (colon)
        if let Some(colon_pos) = input.find(':') {
            let id = &input[..colon_pos];
            if id.is_empty() {
                return Err(ParseError::EmptyId);
            }

            let rest = &input[colon_pos + 1..];
            // Check for index in variant
            if let Some(bracket_pos) = rest.find('[') {
                let variant_child = &rest[..bracket_pos];
                if variant_child.is_empty() {
                    return Err(ParseError::EmptyVariantChild);
                }

                let index_str = &rest[bracket_pos + 1..];
                if !index_str.ends_with(']') {
                    return Err(ParseError::UnclosedBracket);
                }
                let index_str = &index_str[..index_str.len() - 1];
                let index = index_str
                    .parse::<usize>()
                    .map_err(|_| ParseError::InvalidIndex(index_str.to_string()))?;

                Ok(AddressSegment::VariantFilterIndexed {
                    id: id.to_string(),
                    variant_child: variant_child.to_string(),
                    index,
                })
            } else {
                if rest.is_empty() {
                    return Err(ParseError::EmptyVariantChild);
                }
                Ok(AddressSegment::VariantFilter {
                    id: id.to_string(),
                    variant_child: rest.to_string(),
                })
            }
        } else if let Some(bracket_pos) = input.find('[') {
            // Indexed segment
            let id = &input[..bracket_pos];
            if id.is_empty() {
                return Err(ParseError::EmptyId);
            }

            let index_str = &input[bracket_pos + 1..];
            if !index_str.ends_with(']') {
                return Err(ParseError::UnclosedBracket);
            }
            let index_str = &index_str[..index_str.len() - 1];
            let index = index_str
                .parse::<usize>()
                .map_err(|_| ParseError::InvalidIndex(index_str.to_string()))?;

            Ok(AddressSegment::Indexed {
                id: id.to_string(),
                index,
            })
        } else {
            // Simple ID
            if input.is_empty() {
                return Err(ParseError::EmptyId);
            }
            Ok(AddressSegment::Id(input.to_string()))
        }
    }
}

impl fmt::Display for AddressSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AddressSegment::Id(id) => write!(f, "{}", id),
            AddressSegment::Indexed { id, index } => write!(f, "{}[{}]", id, index),
            AddressSegment::VariantFilter { id, variant_child } => {
                write!(f, "{}:{}", id, variant_child)
            }
            AddressSegment::VariantFilterIndexed {
                id,
                variant_child,
                index,
            } => write!(f, "{}:{}[{}]", id, variant_child, index),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Parse simple ID
    // =========================================================================

    #[test]
    fn test_parse_simple_id() {
        let addr = Address::parse("item").unwrap();
        assert_eq!(addr.segments.len(), 1);
        assert_eq!(addr.segments[0], AddressSegment::Id("item".to_string()));
    }

    // =========================================================================
    // Parse indexed
    // =========================================================================

    #[test]
    fn test_parse_indexed() {
        let addr = Address::parse("item[2]").unwrap();
        assert_eq!(addr.segments.len(), 1);
        assert_eq!(
            addr.segments[0],
            AddressSegment::Indexed {
                id: "item".to_string(),
                index: 2
            }
        );
    }

    #[test]
    fn test_parse_indexed_zero() {
        let addr = Address::parse("item[0]").unwrap();
        assert_eq!(
            addr.segments[0],
            AddressSegment::Indexed {
                id: "item".to_string(),
                index: 0
            }
        );
    }

    // =========================================================================
    // Parse with child
    // =========================================================================

    #[test]
    fn test_parse_with_child() {
        let addr = Address::parse("item.delete").unwrap();
        assert_eq!(addr.segments.len(), 2);
        assert_eq!(addr.segments[0], AddressSegment::Id("item".to_string()));
        assert_eq!(addr.segments[1], AddressSegment::Id("delete".to_string()));
    }

    // =========================================================================
    // Parse indexed with child
    // =========================================================================

    #[test]
    fn test_parse_indexed_with_child() {
        let addr = Address::parse("item[2].delete").unwrap();
        assert_eq!(addr.segments.len(), 2);
        assert_eq!(
            addr.segments[0],
            AddressSegment::Indexed {
                id: "item".to_string(),
                index: 2
            }
        );
        assert_eq!(addr.segments[1], AddressSegment::Id("delete".to_string()));
    }

    // =========================================================================
    // Parse variant filter
    // =========================================================================

    #[test]
    fn test_parse_variant_filter() {
        let addr = Address::parse("item:delete").unwrap();
        assert_eq!(addr.segments.len(), 1);
        assert_eq!(
            addr.segments[0],
            AddressSegment::VariantFilter {
                id: "item".to_string(),
                variant_child: "delete".to_string()
            }
        );
    }

    // =========================================================================
    // Parse variant filter indexed
    // =========================================================================

    #[test]
    fn test_parse_variant_filter_indexed() {
        let addr = Address::parse("item:delete[1]").unwrap();
        assert_eq!(addr.segments.len(), 1);
        assert_eq!(
            addr.segments[0],
            AddressSegment::VariantFilterIndexed {
                id: "item".to_string(),
                variant_child: "delete".to_string(),
                index: 1
            }
        );
    }

    // =========================================================================
    // Parse complex
    // =========================================================================

    #[test]
    fn test_parse_complex() {
        let addr = Address::parse("list.item:delete[1].icon").unwrap();
        assert_eq!(addr.segments.len(), 3);
        assert_eq!(addr.segments[0], AddressSegment::Id("list".to_string()));
        assert_eq!(
            addr.segments[1],
            AddressSegment::VariantFilterIndexed {
                id: "item".to_string(),
                variant_child: "delete".to_string(),
                index: 1
            }
        );
        assert_eq!(addr.segments[2], AddressSegment::Id("icon".to_string()));
    }

    #[test]
    fn test_parse_variant_filter_with_child() {
        let addr = Address::parse("item:delete[0].delete").unwrap();
        assert_eq!(addr.segments.len(), 2);
        assert_eq!(
            addr.segments[0],
            AddressSegment::VariantFilterIndexed {
                id: "item".to_string(),
                variant_child: "delete".to_string(),
                index: 0
            }
        );
        assert_eq!(addr.segments[1], AddressSegment::Id("delete".to_string()));
    }

    // =========================================================================
    // Parse errors
    // =========================================================================

    #[test]
    fn test_parse_error_empty_input() {
        assert_eq!(Address::parse(""), Err(ParseError::EmptyInput));
    }

    #[test]
    fn test_parse_error_empty_segment() {
        assert_eq!(Address::parse("item..delete"), Err(ParseError::EmptySegment));
        assert_eq!(Address::parse(".item"), Err(ParseError::EmptySegment));
        assert_eq!(Address::parse("item."), Err(ParseError::EmptySegment));
    }

    #[test]
    fn test_parse_error_invalid_index() {
        assert_eq!(
            Address::parse("item[abc]"),
            Err(ParseError::InvalidIndex("abc".to_string()))
        );
        assert_eq!(
            Address::parse("item[-1]"),
            Err(ParseError::InvalidIndex("-1".to_string()))
        );
    }

    #[test]
    fn test_parse_error_unclosed_bracket() {
        assert_eq!(Address::parse("item[2"), Err(ParseError::UnclosedBracket));
        assert_eq!(
            Address::parse("item:delete[1"),
            Err(ParseError::UnclosedBracket)
        );
    }

    #[test]
    fn test_parse_error_empty_id() {
        assert_eq!(Address::parse("[2]"), Err(ParseError::EmptyId));
        assert_eq!(Address::parse(":delete"), Err(ParseError::EmptyId));
    }

    #[test]
    fn test_parse_error_empty_variant_child() {
        assert_eq!(Address::parse("item:"), Err(ParseError::EmptyVariantChild));
        assert_eq!(Address::parse("item:[1]"), Err(ParseError::EmptyVariantChild));
    }

    // =========================================================================
    // Display
    // =========================================================================

    #[test]
    fn test_display_simple_id() {
        let addr = Address {
            segments: vec![AddressSegment::Id("item".to_string())],
        };
        assert_eq!(addr.to_string(), "item");
    }

    #[test]
    fn test_display_indexed() {
        let addr = Address {
            segments: vec![AddressSegment::Indexed {
                id: "item".to_string(),
                index: 2,
            }],
        };
        assert_eq!(addr.to_string(), "item[2]");
    }

    #[test]
    fn test_display_variant_filter() {
        let addr = Address {
            segments: vec![AddressSegment::VariantFilter {
                id: "item".to_string(),
                variant_child: "delete".to_string(),
            }],
        };
        assert_eq!(addr.to_string(), "item:delete");
    }

    #[test]
    fn test_display_variant_filter_indexed() {
        let addr = Address {
            segments: vec![AddressSegment::VariantFilterIndexed {
                id: "item".to_string(),
                variant_child: "delete".to_string(),
                index: 1,
            }],
        };
        assert_eq!(addr.to_string(), "item:delete[1]");
    }

    #[test]
    fn test_display_complex() {
        let addr = Address {
            segments: vec![
                AddressSegment::Id("list".to_string()),
                AddressSegment::VariantFilterIndexed {
                    id: "item".to_string(),
                    variant_child: "delete".to_string(),
                    index: 1,
                },
                AddressSegment::Id("icon".to_string()),
            ],
        };
        assert_eq!(addr.to_string(), "list.item:delete[1].icon");
    }

    // =========================================================================
    // Roundtrip tests
    // =========================================================================

    #[test]
    fn test_roundtrip_simple_id() {
        let input = "item";
        let addr = Address::parse(input).unwrap();
        assert_eq!(addr.to_string(), input);
    }

    #[test]
    fn test_roundtrip_indexed() {
        let input = "item[2]";
        let addr = Address::parse(input).unwrap();
        assert_eq!(addr.to_string(), input);
    }

    #[test]
    fn test_roundtrip_with_child() {
        let input = "item.delete";
        let addr = Address::parse(input).unwrap();
        assert_eq!(addr.to_string(), input);
    }

    #[test]
    fn test_roundtrip_indexed_with_child() {
        let input = "item[2].delete";
        let addr = Address::parse(input).unwrap();
        assert_eq!(addr.to_string(), input);
    }

    #[test]
    fn test_roundtrip_variant_filter() {
        let input = "item:delete";
        let addr = Address::parse(input).unwrap();
        assert_eq!(addr.to_string(), input);
    }

    #[test]
    fn test_roundtrip_variant_filter_indexed() {
        let input = "item:delete[1]";
        let addr = Address::parse(input).unwrap();
        assert_eq!(addr.to_string(), input);
    }

    #[test]
    fn test_roundtrip_complex() {
        let input = "list.item:delete[1].icon";
        let addr = Address::parse(input).unwrap();
        assert_eq!(addr.to_string(), input);
    }

    #[test]
    fn test_roundtrip_deeply_nested() {
        let input = "app.page[0].section.list.item:edit[2].button";
        let addr = Address::parse(input).unwrap();
        assert_eq!(addr.to_string(), input);
    }
}
