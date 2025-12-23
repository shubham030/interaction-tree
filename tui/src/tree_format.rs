//! Tree Format v2: Compact tree representation with schema deduplication
//!
//! This module provides structures and logic to convert raw `TreeNode` data
//! into a compact format that:
//! - Deduplicates repeated widget structures via schemas
//! - Collapses homogeneous siblings (identical siblings → count)
//! - Groups variants (same ID, different children)
//! - Preserves heterogeneous siblings as positional arrays

use std::collections::{BTreeMap, HashMap};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::app::{ContextInfo, TreeNode};

/// A schema describes a unique InteractionKey definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Schema {
    #[serde(skip)]
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub widget_type: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<String>,
}

impl Schema {
    pub fn from_tree_node(node: &TreeNode) -> Self {
        Self {
            id: node.id.clone(),
            description: None,
            widget_type: node.widget_type.clone(),
            capabilities: Vec::new(),
            actions: Vec::new(),
        }
    }
}

/// A variant represents one pattern of children for a given ID.
/// Used when the same ID appears with different child structures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Variant {
    pub count: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<TreeEntry>,
}

/// Represents different types of tree entries in the compact format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TreeEntry {
    /// Semantic grouping via InteractionContext widget
    Context {
        context: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        children: Vec<TreeEntry>,
    },
    /// Same ID with different child structures
    Variants {
        id: String,
        variants: Vec<Variant>,
    },
    /// Identical siblings collapsed to a count
    Homogeneous {
        id: String,
        count: usize,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        children: Vec<TreeEntry>,
    },
    /// Mixed siblings as positional array (order matters)
    Heterogeneous {
        items: Vec<TreeEntry>,
    },
    /// Single instance
    Singleton {
        id: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        children: Vec<TreeEntry>,
    },
}

impl TreeEntry {
    pub fn id(&self) -> Option<&str> {
        match self {
            TreeEntry::Context { .. } => None,
            TreeEntry::Homogeneous { id, .. } => Some(id),
            TreeEntry::Heterogeneous { .. } => None,
            TreeEntry::Variants { id, .. } => Some(id),
            TreeEntry::Singleton { id, .. } => Some(id),
        }
    }

    fn fmt_tree(&self, f: &mut fmt::Formatter<'_>, prefix: &str, is_last: bool) -> fmt::Result {
        let connector = if is_last { "└── " } else { "├── " };
        let continuation = if is_last { "    " } else { "│   " };

        match self {
            TreeEntry::Context { context, description, children } => {
                write!(f, "{}{}[{}]", prefix, connector, context)?;
                if let Some(desc) = description {
                    write!(f, ": \"{}\"", desc)?;
                }
                writeln!(f)?;
                let child_prefix = format!("{}{}", prefix, continuation);
                for (i, child) in children.iter().enumerate() {
                    child.fmt_tree(f, &child_prefix, i == children.len() - 1)?;
                }
            }
            TreeEntry::Singleton { id, children } => {
                write!(f, "{}{}{}", prefix, connector, id)?;
                if children.len() == 1 {
                    write!(f, " → ")?;
                    children[0].fmt_inline(f)?;
                    writeln!(f)?;
                } else if children.is_empty() {
                    writeln!(f)?;
                } else {
                    writeln!(f)?;
                    let child_prefix = format!("{}{}", prefix, continuation);
                    for (i, child) in children.iter().enumerate() {
                        child.fmt_tree(f, &child_prefix, i == children.len() - 1)?;
                    }
                }
            }
            TreeEntry::Homogeneous {
                id,
                count,
                children,
            } => {
                write!(f, "{}{}{} ×{}", prefix, connector, id, count)?;
                if children.len() == 1 {
                    write!(f, " → ")?;
                    children[0].fmt_inline(f)?;
                    writeln!(f)?;
                } else if children.is_empty() {
                    writeln!(f)?;
                } else {
                    writeln!(f)?;
                    let child_prefix = format!("{}{}", prefix, continuation);
                    for (i, child) in children.iter().enumerate() {
                        child.fmt_tree(f, &child_prefix, i == children.len() - 1)?;
                    }
                }
            }
            TreeEntry::Variants { id, variants } => {
                writeln!(f, "{}{}{}", prefix, connector, id)?;
                let child_prefix = format!("{}{}", prefix, continuation);
                for (i, variant) in variants.iter().enumerate() {
                    let v_is_last = i == variants.len() - 1;
                    let v_connector = if v_is_last { "└── " } else { "├── " };
                    let v_continuation = if v_is_last { "    " } else { "│   " };

                    if variant.children.len() == 1 {
                        write!(f, "{}{}(×{}) → ", child_prefix, v_connector, variant.count)?;
                        variant.children[0].fmt_inline(f)?;
                        writeln!(f)?;
                    } else if variant.children.is_empty() {
                        writeln!(f, "{}{}(×{})", child_prefix, v_connector, variant.count)?;
                    } else {
                        writeln!(f, "{}{}(×{})", child_prefix, v_connector, variant.count)?;
                        let v_child_prefix = format!("{}{}", child_prefix, v_continuation);
                        for (j, child) in variant.children.iter().enumerate() {
                            child.fmt_tree(f, &v_child_prefix, j == variant.children.len() - 1)?;
                        }
                    }
                }
            }
            TreeEntry::Heterogeneous { items } => {
                for (i, item) in items.iter().enumerate() {
                    item.fmt_tree(f, prefix, i == items.len() - 1)?;
                }
            }
        }
        Ok(())
    }

    fn fmt_inline(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TreeEntry::Singleton { id, children } => {
                write!(f, "{}", id)?;
                if children.len() == 1 {
                    write!(f, " → ")?;
                    children[0].fmt_inline(f)?;
                }
            }
            TreeEntry::Homogeneous {
                id,
                count,
                children,
            } => {
                write!(f, "{} ×{}", id, count)?;
                if children.len() == 1 {
                    write!(f, " → ")?;
                    children[0].fmt_inline(f)?;
                }
            }
            TreeEntry::Context { context, description, .. } => {
                write!(f, "[{}]", context)?;
                if let Some(desc) = description {
                    write!(f, ": \"{}\"", desc)?;
                }
            }
            TreeEntry::Variants { id, .. } => {
                write!(f, "{}", id)?;
            }
            TreeEntry::Heterogeneous { .. } => {
                write!(f, "[...]")?;
            }
        }
        Ok(())
    }
}

/// The top-level compact tree structure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompactTree {
    /// Schema definitions keyed by ID
    pub schemas: BTreeMap<String, Schema>,
    /// The compacted tree entries
    pub tree: Vec<TreeEntry>,
}

impl CompactTree {
    pub fn new() -> Self {
        Self {
            schemas: BTreeMap::new(),
            tree: Vec::new(),
        }
    }

    /// Convert raw TreeNodes into a CompactTree
    pub fn from_tree_nodes(nodes: &[TreeNode]) -> (Self, Vec<SchemaWarning>) {
        let mut extractor = SchemaExtractor::new();
        extractor.extract_from_nodes(nodes);

        // Build context-grouped tree
        let tree = build_context_tree(nodes);

        (
            CompactTree {
                schemas: extractor.schemas,
                tree,
            },
            extractor.warnings,
        )
    }

    /// Convert to printable format for LLM consumption
    #[allow(dead_code)]
    pub fn to_printable(&self) -> String {
        format!("{}", self)
    }
}

impl fmt::Display for CompactTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Schemas section
        if !self.schemas.is_empty() {
            writeln!(f, "[Schemas]")?;
            for (id, schema) in &self.schemas {
                write!(f, "• {}", id)?;
                if let Some(desc) = &schema.description {
                    write!(f, ": \"{}\"", desc)?;
                }
                if let Some(wt) = &schema.widget_type {
                    write!(f, " ({})", wt)?;
                }
                if !schema.capabilities.is_empty() {
                    write!(f, " [{}]", schema.capabilities.join(", "))?;
                }
                if !schema.actions.is_empty() {
                    write!(f, " {{{}}}", schema.actions.join(", "))?;
                }
                writeln!(f)?;
            }
            writeln!(f)?;
        }

        // Tree section
        if !self.tree.is_empty() {
            writeln!(f, "[Tree]")?;
            for (i, entry) in self.tree.iter().enumerate() {
                let is_last = i == self.tree.len() - 1;
                entry.fmt_tree(f, "", is_last)?;
            }
        }

        Ok(())
    }
}

impl Default for CompactTree {
    fn default() -> Self {
        Self::new()
    }
}

/// Warning emitted when schema extraction finds inconsistencies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaWarning {
    pub id: String,
    pub message: String,
}

/// Extracts unique schemas from tree nodes.
struct SchemaExtractor {
    schemas: BTreeMap<String, Schema>,
    warnings: Vec<SchemaWarning>,
}

impl SchemaExtractor {
    fn new() -> Self {
        Self {
            schemas: BTreeMap::new(),
            warnings: Vec::new(),
        }
    }

    fn extract_from_nodes(&mut self, nodes: &[TreeNode]) {
        for node in nodes {
            self.extract_from_node(node);
        }
    }

    fn extract_from_node(&mut self, node: &TreeNode) {
        let new_schema = Schema::from_tree_node(node);

        if let Some(existing) = self.schemas.get(&node.id) {
            // Check for mismatches (widget_type for now, description when available)
            if existing.widget_type != new_schema.widget_type {
                self.warnings.push(SchemaWarning {
                    id: node.id.clone(),
                    message: format!(
                        "widget_type mismatch: first={:?}, new={:?}",
                        existing.widget_type, new_schema.widget_type
                    ),
                });
            }
            // Keep first seen (don't update)
        } else {
            self.schemas.insert(node.id.clone(), new_schema);
        }

        // Recurse into children
        for child in &node.children {
            self.extract_from_node(child);
        }
    }
}

/// Compute a fingerprint for a node's child structure.
/// This is used to detect variants (same ID, different children patterns).
fn child_fingerprint(node: &TreeNode) -> String {
    if node.children.is_empty() {
        return String::new();
    }

    let child_ids: Vec<&str> = node.children.iter().map(|c| c.id.as_str()).collect();
    child_ids.join(",")
}

/// Build a context-grouped tree from flat nodes.
/// Nodes are grouped under their InteractionContext ancestors.
/// Preserves original order by tracking first occurrence of each context path.
fn build_context_tree(nodes: &[TreeNode]) -> Vec<TreeEntry> {
    if nodes.is_empty() {
        return Vec::new();
    }

    // Track context paths in order of first occurrence
    let mut context_map: HashMap<Vec<String>, (Vec<ContextInfo>, Vec<&TreeNode>)> = HashMap::new();
    
    // Track slots to preserve interleaving of no-context and context nodes
    #[derive(Clone)]
    enum TreeSlot {
        NoContext(Vec<TreeNode>),
        Context(Vec<String>),
    }
    let mut slots: Vec<TreeSlot> = Vec::new();
    let mut current_no_context: Vec<TreeNode> = Vec::new();

    for node in nodes {
        if node.contexts.is_empty() {
            current_no_context.push(node.clone());
        } else {
            // Flush any accumulated no-context nodes
            if !current_no_context.is_empty() {
                slots.push(TreeSlot::NoContext(std::mem::take(&mut current_no_context)));
            }
            
            let context_path: Vec<String> = node.contexts.iter().map(|c| c.name.clone()).collect();
            
            // Track order of first occurrence
            if !context_map.contains_key(&context_path) {
                slots.push(TreeSlot::Context(context_path.clone()));
            }
            
            let entry = context_map.entry(context_path).or_insert_with(|| (node.contexts.clone(), Vec::new()));
            entry.1.push(node);
        }
    }
    
    // Flush remaining no-context nodes
    if !current_no_context.is_empty() {
        slots.push(TreeSlot::NoContext(current_no_context));
    }

    let mut result = Vec::new();

    // Process slots in order to preserve interleaving
    for slot in slots {
        match slot {
            TreeSlot::NoContext(nodes) => {
                result.extend(compact_siblings(&nodes));
            }
            TreeSlot::Context(context_path) => {
                if let Some((contexts, nodes_in_context)) = context_map.remove(&context_path) {
                    let owned_nodes: Vec<TreeNode> = nodes_in_context.iter().map(|n| (*n).clone()).collect();
                    let children = compact_siblings(&owned_nodes);
                    let entry = build_nested_contexts(&contexts, children);
                    result.push(entry);
                }
            }
        }
    }

    result
}

/// Build nested TreeEntry::Context from a list of contexts (outermost first).
fn build_nested_contexts(contexts: &[ContextInfo], innermost_children: Vec<TreeEntry>) -> TreeEntry {
    if contexts.is_empty() {
        // Shouldn't happen, but return a heterogeneous entry as fallback
        return TreeEntry::Heterogeneous { items: innermost_children };
    }

    // Build from innermost to outermost
    let mut current_children = innermost_children;

    for ctx in contexts.iter().rev() {
        current_children = vec![TreeEntry::Context {
            context: ctx.name.clone(),
            description: ctx.description.clone(),
            children: current_children,
        }];
    }

    // Return the outermost context (unwrap the vec since we always have exactly one)
    current_children.into_iter().next().unwrap()
}

/// Compact a list of sibling nodes into TreeEntries.
fn compact_siblings(nodes: &[TreeNode]) -> Vec<TreeEntry> {
    if nodes.is_empty() {
        return Vec::new();
    }

    // Group consecutive nodes by ID
    let groups = group_by_id(nodes);

    if groups.len() == 1 && groups[0].len() == nodes.len() {
        // All nodes have the same ID - check if homogeneous or variants
        let group = &groups[0];
        return compact_same_id_group(group);
    }

    // Multiple different IDs - check if we can simplify
    if groups.len() == nodes.len() {
        // Each node is unique - could still be heterogeneous if order matters
        // For now, emit as individual entries
        let entries: Vec<TreeEntry> = nodes.iter().map(node_to_entry).collect();

        // If all entries are singletons with the same parent context, just return them
        return entries;
    }

    // Mixed case: some repeated, some unique
    let mut result = Vec::new();
    for group in groups {
        let entries = compact_same_id_group(&group);
        result.extend(entries);
    }
    result
}

/// Group consecutive nodes by their ID.
fn group_by_id(nodes: &[TreeNode]) -> Vec<Vec<&TreeNode>> {
    let mut groups: Vec<Vec<&TreeNode>> = Vec::new();

    for node in nodes {
        if let Some(last_group) = groups.last_mut() {
            if last_group[0].id == node.id {
                last_group.push(node);
                continue;
            }
        }
        groups.push(vec![node]);
    }

    groups
}

/// Compact a group of nodes that all have the same ID.
fn compact_same_id_group(nodes: &[&TreeNode]) -> Vec<TreeEntry> {
    if nodes.is_empty() {
        return Vec::new();
    }

    if nodes.len() == 1 {
        return vec![node_to_entry(nodes[0])];
    }

    let id = &nodes[0].id;

    // Group by child fingerprint to detect variants
    let mut by_fingerprint: HashMap<String, Vec<&TreeNode>> = HashMap::new();
    for node in nodes {
        let fp = child_fingerprint(node);
        by_fingerprint.entry(fp).or_default().push(node);
    }

    if by_fingerprint.len() == 1 {
        // All have the same child structure - homogeneous
        let representative = nodes[0];
        let children = compact_siblings(&representative.children);

        vec![TreeEntry::Homogeneous {
            id: id.clone(),
            count: nodes.len(),
            children,
        }]
    } else {
        // Different child structures - variants
        let mut variants: Vec<Variant> = Vec::new();

        for (_, group) in by_fingerprint {
            let representative = group[0];
            let children = compact_siblings(&representative.children);
            variants.push(Variant {
                count: group.len(),
                children,
            });
        }

        // Sort variants by count (descending) for consistency
        variants.sort_by(|a, b| b.count.cmp(&a.count));

        vec![TreeEntry::Variants {
            id: id.clone(),
            variants,
        }]
    }
}

/// Convert a single TreeNode to a TreeEntry.
fn node_to_entry(node: &TreeNode) -> TreeEntry {
    let children = compact_siblings(&node.children);

    TreeEntry::Singleton {
        id: node.id.clone(),
        children,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(id: &str, children: Vec<TreeNode>) -> TreeNode {
        TreeNode {
            id: id.to_string(),
            widget_type: None,
            capabilities: Vec::new(),
            actions: Vec::new(),
            children,
            contexts: Vec::new(),
        }
    }

    fn make_node_with_type(id: &str, widget_type: &str, children: Vec<TreeNode>) -> TreeNode {
        TreeNode {
            id: id.to_string(),
            widget_type: Some(widget_type.to_string()),
            capabilities: Vec::new(),
            actions: Vec::new(),
            children,
            contexts: Vec::new(),
        }
    }

    fn make_node_with_context(id: &str, contexts: Vec<ContextInfo>, children: Vec<TreeNode>) -> TreeNode {
        TreeNode {
            id: id.to_string(),
            widget_type: None,
            capabilities: Vec::new(),
            actions: Vec::new(),
            children,
            contexts,
        }
    }

    // =========================================================================
    // Schema extraction tests
    // =========================================================================

    #[test]
    fn test_schema_extraction_basic() {
        let nodes = vec![
            make_node_with_type("item", "ListTile", vec![]),
            make_node_with_type("delete", "IconButton", vec![]),
        ];

        let (tree, warnings) = CompactTree::from_tree_nodes(&nodes);

        assert!(warnings.is_empty());
        assert_eq!(tree.schemas.len(), 2);

        let item_schema = tree.schemas.get("item").unwrap();
        assert_eq!(item_schema.widget_type, Some("ListTile".to_string()));

        let delete_schema = tree.schemas.get("delete").unwrap();
        assert_eq!(delete_schema.widget_type, Some("IconButton".to_string()));
    }

    #[test]
    fn test_schema_extraction_nested() {
        let nodes = vec![make_node_with_type(
            "list",
            "ListView",
            vec![
                make_node_with_type("item", "ListTile", vec![]),
                make_node_with_type("item", "ListTile", vec![]),
            ],
        )];

        let (tree, warnings) = CompactTree::from_tree_nodes(&nodes);

        assert!(warnings.is_empty());
        assert_eq!(tree.schemas.len(), 2);
        assert!(tree.schemas.contains_key("list"));
        assert!(tree.schemas.contains_key("item"));
    }

    #[test]
    fn test_schema_extraction_consistent_descriptions() {
        // Same ID with same widget_type should not warn
        let nodes = vec![
            make_node_with_type("btn", "ElevatedButton", vec![]),
            make_node_with_type("btn", "ElevatedButton", vec![]),
            make_node_with_type("btn", "ElevatedButton", vec![]),
        ];

        let (tree, warnings) = CompactTree::from_tree_nodes(&nodes);

        assert!(warnings.is_empty());
        assert_eq!(tree.schemas.len(), 1);
        assert_eq!(
            tree.schemas.get("btn").unwrap().widget_type,
            Some("ElevatedButton".to_string())
        );
    }

    #[test]
    fn test_schema_extraction_mismatched_widget_type() {
        // Same ID with different widget_type should warn and use first
        let nodes = vec![
            make_node_with_type("action", "IconButton", vec![]),
            make_node_with_type("action", "TextButton", vec![]),
        ];

        let (tree, warnings) = CompactTree::from_tree_nodes(&nodes);

        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].id, "action");
        assert!(warnings[0].message.contains("widget_type mismatch"));

        // First seen should be kept
        assert_eq!(
            tree.schemas.get("action").unwrap().widget_type,
            Some("IconButton".to_string())
        );
    }

    #[test]
    fn test_schema_extraction_multiple_mismatches() {
        let nodes = vec![
            make_node_with_type("a", "TypeA1", vec![]),
            make_node_with_type("b", "TypeB1", vec![]),
            make_node_with_type("a", "TypeA2", vec![]),
            make_node_with_type("b", "TypeB2", vec![]),
        ];

        let (tree, warnings) = CompactTree::from_tree_nodes(&nodes);

        assert_eq!(warnings.len(), 2);
        assert_eq!(tree.schemas.len(), 2);

        // First seen preserved
        assert_eq!(
            tree.schemas.get("a").unwrap().widget_type,
            Some("TypeA1".to_string())
        );
        assert_eq!(
            tree.schemas.get("b").unwrap().widget_type,
            Some("TypeB1".to_string())
        );
    }

    // =========================================================================
    // Homogeneous sibling detection tests
    // =========================================================================

    #[test]
    fn test_homogeneous_siblings_simple() {
        // 5 identical "item" nodes with no children
        let nodes: Vec<TreeNode> = (0..5).map(|_| make_node("item", vec![])).collect();

        let (tree, _) = CompactTree::from_tree_nodes(&nodes);

        assert_eq!(tree.tree.len(), 1);
        match &tree.tree[0] {
            TreeEntry::Homogeneous { id, count, children } => {
                assert_eq!(id, "item");
                assert_eq!(*count, 5);
                assert!(children.is_empty());
            }
            _ => panic!("Expected Homogeneous entry"),
        }
    }

    #[test]
    fn test_homogeneous_siblings_with_children() {
        // 3 "item" nodes each with a "delete" child
        let nodes: Vec<TreeNode> = (0..3)
            .map(|_| make_node("item", vec![make_node("delete", vec![])]))
            .collect();

        let (tree, _) = CompactTree::from_tree_nodes(&nodes);

        assert_eq!(tree.tree.len(), 1);
        match &tree.tree[0] {
            TreeEntry::Homogeneous { id, count, children } => {
                assert_eq!(id, "item");
                assert_eq!(*count, 3);
                assert_eq!(children.len(), 1);

                // Child should be singleton since structure is compacted
                match &children[0] {
                    TreeEntry::Singleton { id, .. } => {
                        assert_eq!(id, "delete");
                    }
                    _ => panic!("Expected Singleton child"),
                }
            }
            _ => panic!("Expected Homogeneous entry"),
        }
    }

    #[test]
    fn test_homogeneous_deep_nesting() {
        // item → action → icon (all homogeneous)
        let nodes: Vec<TreeNode> = (0..2)
            .map(|_| {
                make_node(
                    "item",
                    vec![make_node("action", vec![make_node("icon", vec![])])],
                )
            })
            .collect();

        let (tree, _) = CompactTree::from_tree_nodes(&nodes);

        assert_eq!(tree.tree.len(), 1);
        match &tree.tree[0] {
            TreeEntry::Homogeneous {
                id,
                count,
                children,
            } => {
                assert_eq!(id, "item");
                assert_eq!(*count, 2);

                // Verify nested structure
                match &children[0] {
                    TreeEntry::Singleton { id, children } => {
                        assert_eq!(id, "action");
                        match &children[0] {
                            TreeEntry::Singleton { id, .. } => {
                                assert_eq!(id, "icon");
                            }
                            _ => panic!("Expected nested Singleton"),
                        }
                    }
                    _ => panic!("Expected Singleton child"),
                }
            }
            _ => panic!("Expected Homogeneous entry"),
        }
    }

    // =========================================================================
    // Variant grouping tests
    // =========================================================================

    #[test]
    fn test_variants_different_children() {
        // Same ID "item" but with different child structures
        let nodes = vec![
            make_node("item", vec![make_node("delete", vec![])]),
            make_node("item", vec![make_node("delete", vec![])]),
            make_node("item", vec![make_node("edit", vec![])]),
        ];

        let (tree, _) = CompactTree::from_tree_nodes(&nodes);

        assert_eq!(tree.tree.len(), 1);
        match &tree.tree[0] {
            TreeEntry::Variants { id, variants } => {
                assert_eq!(id, "item");
                assert_eq!(variants.len(), 2);

                // Sorted by count descending
                assert_eq!(variants[0].count, 2);
                assert_eq!(variants[1].count, 1);
            }
            _ => panic!("Expected Variants entry"),
        }
    }

    #[test]
    fn test_variants_some_with_children_some_without() {
        let nodes = vec![
            make_node("item", vec![]),
            make_node("item", vec![make_node("action", vec![])]),
            make_node("item", vec![]),
        ];

        let (tree, _) = CompactTree::from_tree_nodes(&nodes);

        assert_eq!(tree.tree.len(), 1);
        match &tree.tree[0] {
            TreeEntry::Variants { id, variants } => {
                assert_eq!(id, "item");
                assert_eq!(variants.len(), 2);

                // 2 without children, 1 with children
                let empty_count = variants.iter().find(|v| v.children.is_empty());
                let with_children = variants.iter().find(|v| !v.children.is_empty());

                assert_eq!(empty_count.unwrap().count, 2);
                assert_eq!(with_children.unwrap().count, 1);
            }
            _ => panic!("Expected Variants entry"),
        }
    }

    #[test]
    fn test_variants_multiple_different_patterns() {
        let nodes = vec![
            make_node("card", vec![make_node("title", vec![])]),
            make_node("card", vec![make_node("title", vec![]), make_node("subtitle", vec![])]),
            make_node("card", vec![make_node("title", vec![]), make_node("subtitle", vec![])]),
            make_node("card", vec![make_node("image", vec![])]),
        ];

        let (tree, _) = CompactTree::from_tree_nodes(&nodes);

        assert_eq!(tree.tree.len(), 1);
        match &tree.tree[0] {
            TreeEntry::Variants { id, variants } => {
                assert_eq!(id, "card");
                assert_eq!(variants.len(), 3);

                // title,subtitle appears twice
                assert_eq!(variants[0].count, 2);
            }
            _ => panic!("Expected Variants entry"),
        }
    }

    // =========================================================================
    // Heterogeneous detection tests
    // =========================================================================

    #[test]
    fn test_heterogeneous_siblings() {
        // Different IDs - currently returns as individual singletons
        let nodes = vec![
            make_node("header", vec![]),
            make_node("content", vec![]),
            make_node("footer", vec![]),
        ];

        let (tree, _) = CompactTree::from_tree_nodes(&nodes);

        assert_eq!(tree.tree.len(), 3);
        for entry in &tree.tree {
            match entry {
                TreeEntry::Singleton { .. } => {}
                _ => panic!("Expected Singleton entries for heterogeneous siblings"),
            }
        }
    }

    #[test]
    fn test_singleton() {
        let nodes = vec![make_node("root", vec![make_node("child", vec![])])];

        let (tree, _) = CompactTree::from_tree_nodes(&nodes);

        assert_eq!(tree.tree.len(), 1);
        match &tree.tree[0] {
            TreeEntry::Singleton { id, children } => {
                assert_eq!(id, "root");
                assert_eq!(children.len(), 1);
            }
            _ => panic!("Expected Singleton entry"),
        }
    }

    // =========================================================================
    // Mixed scenario tests
    // =========================================================================

    #[test]
    fn test_mixed_homogeneous_and_heterogeneous() {
        // header, item×3, footer
        let nodes = vec![
            make_node("header", vec![]),
            make_node("item", vec![]),
            make_node("item", vec![]),
            make_node("item", vec![]),
            make_node("footer", vec![]),
        ];

        let (tree, _) = CompactTree::from_tree_nodes(&nodes);

        // Should be: header singleton, item homogeneous×3, footer singleton
        assert_eq!(tree.tree.len(), 3);

        match &tree.tree[0] {
            TreeEntry::Singleton { id, .. } => assert_eq!(id, "header"),
            _ => panic!("Expected header Singleton"),
        }

        match &tree.tree[1] {
            TreeEntry::Homogeneous { id, count, .. } => {
                assert_eq!(id, "item");
                assert_eq!(*count, 3);
            }
            _ => panic!("Expected item Homogeneous"),
        }

        match &tree.tree[2] {
            TreeEntry::Singleton { id, .. } => assert_eq!(id, "footer"),
            _ => panic!("Expected footer Singleton"),
        }
    }

    #[test]
    fn test_complex_nested_structure() {
        // list → [item×2 → delete, item×1 → edit]
        let nodes = vec![make_node(
            "list",
            vec![
                make_node("item", vec![make_node("delete", vec![])]),
                make_node("item", vec![make_node("delete", vec![])]),
                make_node("item", vec![make_node("edit", vec![])]),
            ],
        )];

        let (tree, _) = CompactTree::from_tree_nodes(&nodes);

        assert_eq!(tree.tree.len(), 1);

        match &tree.tree[0] {
            TreeEntry::Singleton { id, children } => {
                assert_eq!(id, "list");
                assert_eq!(children.len(), 1);

                // Children should be variants since same ID with different children
                match &children[0] {
                    TreeEntry::Variants { id, variants } => {
                        assert_eq!(id, "item");
                        assert_eq!(variants.len(), 2);
                    }
                    _ => panic!("Expected Variants for items"),
                }
            }
            _ => panic!("Expected Singleton for list"),
        }
    }

    #[test]
    fn test_empty_input() {
        let nodes: Vec<TreeNode> = vec![];
        let (tree, warnings) = CompactTree::from_tree_nodes(&nodes);

        assert!(tree.schemas.is_empty());
        assert!(tree.tree.is_empty());
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_deeply_nested_homogeneous() {
        // 3 levels of homogeneous nesting
        let nodes: Vec<TreeNode> = (0..2)
            .map(|_| {
                make_node(
                    "a",
                    (0..3)
                        .map(|_| {
                            make_node("b", (0..4).map(|_| make_node("c", vec![])).collect())
                        })
                        .collect(),
                )
            })
            .collect();

        let (tree, _) = CompactTree::from_tree_nodes(&nodes);

        // Top level: a×2
        assert_eq!(tree.tree.len(), 1);
        match &tree.tree[0] {
            TreeEntry::Homogeneous { id, count, children } => {
                assert_eq!(id, "a");
                assert_eq!(*count, 2);

                // Second level: b×3
                match &children[0] {
                    TreeEntry::Homogeneous {
                        id,
                        count,
                        children,
                    } => {
                        assert_eq!(id, "b");
                        assert_eq!(*count, 3);

                        // Third level: c×4
                        match &children[0] {
                            TreeEntry::Homogeneous { id, count, .. } => {
                                assert_eq!(id, "c");
                                assert_eq!(*count, 4);
                            }
                            _ => panic!("Expected c Homogeneous"),
                        }
                    }
                    _ => panic!("Expected b Homogeneous"),
                }
            }
            _ => panic!("Expected a Homogeneous"),
        }

        // Schema extraction should find all 3 IDs
        assert_eq!(tree.schemas.len(), 3);
        assert!(tree.schemas.contains_key("a"));
        assert!(tree.schemas.contains_key("b"));
        assert!(tree.schemas.contains_key("c"));
    }

    // =========================================================================
    // Helper function tests
    // =========================================================================

    #[test]
    fn test_child_fingerprint() {
        let node_empty = make_node("x", vec![]);
        assert_eq!(child_fingerprint(&node_empty), "");

        let node_one = make_node("x", vec![make_node("a", vec![])]);
        assert_eq!(child_fingerprint(&node_one), "a");

        let node_multi = make_node(
            "x",
            vec![
                make_node("a", vec![]),
                make_node("b", vec![]),
                make_node("c", vec![]),
            ],
        );
        assert_eq!(child_fingerprint(&node_multi), "a,b,c");
    }

    #[test]
    fn test_group_by_id() {
        let nodes = vec![
            make_node("a", vec![]),
            make_node("a", vec![]),
            make_node("b", vec![]),
            make_node("a", vec![]),
        ];

        let groups = group_by_id(&nodes);

        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].len(), 2); // a, a
        assert_eq!(groups[1].len(), 1); // b
        assert_eq!(groups[2].len(), 1); // a
    }

    #[test]
    fn test_tree_entry_id() {
        let singleton = TreeEntry::Singleton {
            id: "test".to_string(),
            children: vec![],
        };
        assert_eq!(singleton.id(), Some("test"));

        let homogeneous = TreeEntry::Homogeneous {
            id: "item".to_string(),
            count: 5,
            children: vec![],
        };
        assert_eq!(homogeneous.id(), Some("item"));

        let context = TreeEntry::Context {
            context: "Cart".to_string(),
            description: None,
            children: vec![],
        };
        assert_eq!(context.id(), None);

        let heterogeneous = TreeEntry::Heterogeneous { items: vec![] };
        assert_eq!(heterogeneous.id(), None);
    }

    // =========================================================================
    // JSON Serialization tests
    // =========================================================================

    #[test]
    fn test_json_roundtrip_simple() {
        let tree = CompactTree {
            schemas: BTreeMap::from([(
                "item".to_string(),
                Schema {
                    id: "item".to_string(),
                    description: Some("A list item".to_string()),
                    widget_type: Some("ListTile".to_string()),
                    capabilities: vec!["tap".to_string()],
                    actions: vec![],
                },
            )]),
            tree: vec![TreeEntry::Homogeneous {
                id: "item".to_string(),
                count: 5,
                children: vec![],
            }],
        };

        let json = serde_json::to_string(&tree).unwrap();
        let parsed: CompactTree = serde_json::from_str(&json).unwrap();

        assert_eq!(tree.tree, parsed.tree);
        assert_eq!(
            tree.schemas.get("item").unwrap().description,
            parsed.schemas.get("item").unwrap().description
        );
    }

    #[test]
    fn test_json_roundtrip_nested() {
        let tree = CompactTree {
            schemas: BTreeMap::new(),
            tree: vec![TreeEntry::Singleton {
                id: "list".to_string(),
                children: vec![TreeEntry::Homogeneous {
                    id: "item".to_string(),
                    count: 3,
                    children: vec![TreeEntry::Singleton {
                        id: "delete".to_string(),
                        children: vec![],
                    }],
                }],
            }],
        };

        let json = serde_json::to_string(&tree).unwrap();
        let parsed: CompactTree = serde_json::from_str(&json).unwrap();

        assert_eq!(tree, parsed);
    }

    #[test]
    fn test_json_output_structure() {
        let tree = CompactTree {
            schemas: BTreeMap::from([(
                "item".to_string(),
                Schema {
                    id: "item".to_string(),
                    description: Some("A list item".to_string()),
                    widget_type: Some("ListTile".to_string()),
                    capabilities: vec!["tap".to_string()],
                    actions: vec![],
                },
            )]),
            tree: vec![TreeEntry::Homogeneous {
                id: "item".to_string(),
                count: 5,
                children: vec![TreeEntry::Singleton {
                    id: "delete".to_string(),
                    children: vec![],
                }],
            }],
        };

        let json: serde_json::Value = serde_json::to_value(&tree).unwrap();

        assert!(json["schemas"]["item"]["description"]
            .as_str()
            .unwrap()
            .contains("list item"));
        assert_eq!(json["schemas"]["item"]["widgetType"], "ListTile");
        assert_eq!(json["schemas"]["item"]["capabilities"][0], "tap");

        assert_eq!(json["tree"][0]["id"], "item");
        assert_eq!(json["tree"][0]["count"], 5);
        assert_eq!(json["tree"][0]["children"][0]["id"], "delete");
    }

    #[test]
    fn test_json_variants_structure() {
        let tree = CompactTree {
            schemas: BTreeMap::new(),
            tree: vec![TreeEntry::Variants {
                id: "item".to_string(),
                variants: vec![
                    Variant {
                        count: 3,
                        children: vec![TreeEntry::Singleton {
                            id: "delete".to_string(),
                            children: vec![],
                        }],
                    },
                    Variant {
                        count: 2,
                        children: vec![TreeEntry::Singleton {
                            id: "edit".to_string(),
                            children: vec![],
                        }],
                    },
                ],
            }],
        };

        let json: serde_json::Value = serde_json::to_value(&tree).unwrap();

        assert_eq!(json["tree"][0]["id"], "item");
        assert_eq!(json["tree"][0]["variants"][0]["count"], 3);
        assert_eq!(json["tree"][0]["variants"][0]["children"][0]["id"], "delete");
        assert_eq!(json["tree"][0]["variants"][1]["count"], 2);
    }

    // =========================================================================
    // Printable format tests
    // =========================================================================

    #[test]
    fn test_printable_simple_homogeneous() {
        let tree = CompactTree {
            schemas: BTreeMap::from([(
                "item".to_string(),
                Schema {
                    id: "item".to_string(),
                    description: Some("A list item".to_string()),
                    widget_type: None,
                    capabilities: vec!["tap".to_string()],
                    actions: vec![],
                },
            )]),
            tree: vec![TreeEntry::Homogeneous {
                id: "item".to_string(),
                count: 5,
                children: vec![],
            }],
        };

        let output = tree.to_printable();

        let expected = "\
[Schemas]
• item: \"A list item\" [tap]

[Tree]
└── item ×5
";
        assert_eq!(output, expected);
    }

    #[test]
    fn test_printable_homogeneous_with_single_child() {
        let tree = CompactTree {
            schemas: BTreeMap::new(),
            tree: vec![TreeEntry::Homogeneous {
                id: "item".to_string(),
                count: 5,
                children: vec![TreeEntry::Singleton {
                    id: "delete".to_string(),
                    children: vec![],
                }],
            }],
        };

        let output = tree.to_printable();

        let expected = "\
[Tree]
└── item ×5 → delete
";
        assert_eq!(output, expected);
    }

    #[test]
    fn test_printable_variants() {
        let tree = CompactTree {
            schemas: BTreeMap::new(),
            tree: vec![TreeEntry::Variants {
                id: "item".to_string(),
                variants: vec![
                    Variant {
                        count: 3,
                        children: vec![TreeEntry::Singleton {
                            id: "delete".to_string(),
                            children: vec![],
                        }],
                    },
                    Variant {
                        count: 2,
                        children: vec![TreeEntry::Singleton {
                            id: "edit".to_string(),
                            children: vec![],
                        }],
                    },
                ],
            }],
        };

        let output = tree.to_printable();

        let expected = "\
[Tree]
└── item
    ├── (×3) → delete
    └── (×2) → edit
";
        assert_eq!(output, expected);
    }

    #[test]
    fn test_printable_nested_structure() {
        let tree = CompactTree {
            schemas: BTreeMap::new(),
            tree: vec![TreeEntry::Singleton {
                id: "list".to_string(),
                children: vec![TreeEntry::Homogeneous {
                    id: "item".to_string(),
                    count: 3,
                    children: vec![TreeEntry::Singleton {
                        id: "delete".to_string(),
                        children: vec![],
                    }],
                }],
            }],
        };

        let output = tree.to_printable();

        let expected = "\
[Tree]
└── list → item ×3 → delete
";
        assert_eq!(output, expected);
    }

    #[test]
    fn test_printable_mixed_scenario() {
        let tree = CompactTree {
            schemas: BTreeMap::from([
                (
                    "header".to_string(),
                    Schema {
                        id: "header".to_string(),
                        description: Some("Page header".to_string()),
                        widget_type: Some("AppBar".to_string()),
                        capabilities: vec![],
                        actions: vec![],
                    },
                ),
                (
                    "item".to_string(),
                    Schema {
                        id: "item".to_string(),
                        description: Some("List item".to_string()),
                        widget_type: Some("ListTile".to_string()),
                        capabilities: vec!["tap".to_string(), "longPress".to_string()],
                        actions: vec!["delete".to_string()],
                    },
                ),
            ]),
            tree: vec![
                TreeEntry::Singleton {
                    id: "header".to_string(),
                    children: vec![],
                },
                TreeEntry::Homogeneous {
                    id: "item".to_string(),
                    count: 5,
                    children: vec![TreeEntry::Singleton {
                        id: "delete".to_string(),
                        children: vec![],
                    }],
                },
            ],
        };

        let output = tree.to_printable();

        let expected = "\
[Schemas]
• header: \"Page header\" (AppBar)
• item: \"List item\" (ListTile) [tap, longPress] {delete}

[Tree]
├── header
└── item ×5 → delete
";
        assert_eq!(output, expected);
    }

    #[test]
    fn test_printable_multiple_children() {
        let tree = CompactTree {
            schemas: BTreeMap::new(),
            tree: vec![TreeEntry::Singleton {
                id: "card".to_string(),
                children: vec![
                    TreeEntry::Singleton {
                        id: "title".to_string(),
                        children: vec![],
                    },
                    TreeEntry::Singleton {
                        id: "subtitle".to_string(),
                        children: vec![],
                    },
                    TreeEntry::Singleton {
                        id: "action".to_string(),
                        children: vec![],
                    },
                ],
            }],
        };

        let output = tree.to_printable();

        let expected = "\
[Tree]
└── card
    ├── title
    ├── subtitle
    └── action
";
        assert_eq!(output, expected);
    }

    #[test]
    fn test_printable_deep_nested_lines() {
        let tree = CompactTree {
            schemas: BTreeMap::new(),
            tree: vec![TreeEntry::Singleton {
                id: "page".to_string(),
                children: vec![
                    TreeEntry::Singleton {
                        id: "header".to_string(),
                        children: vec![
                            TreeEntry::Singleton {
                                id: "logo".to_string(),
                                children: vec![],
                            },
                            TreeEntry::Singleton {
                                id: "nav".to_string(),
                                children: vec![],
                            },
                        ],
                    },
                    TreeEntry::Singleton {
                        id: "content".to_string(),
                        children: vec![TreeEntry::Homogeneous {
                            id: "item".to_string(),
                            count: 5,
                            children: vec![TreeEntry::Singleton {
                                id: "delete".to_string(),
                                children: vec![],
                            }],
                        }],
                    },
                    TreeEntry::Singleton {
                        id: "footer".to_string(),
                        children: vec![
                            TreeEntry::Singleton {
                                id: "links".to_string(),
                                children: vec![],
                            },
                            TreeEntry::Singleton {
                                id: "copyright".to_string(),
                                children: vec![],
                            },
                        ],
                    },
                ],
            }],
        };

        let output = tree.to_printable();

        let expected = "\
[Tree]
└── page
    ├── header
    │   ├── logo
    │   └── nav
    ├── content → item ×5 → delete
    └── footer
        ├── links
        └── copyright
";

        assert_eq!(output, expected);
    }

    // =========================================================================
    // Context grouping tests
    // =========================================================================

    #[test]
    fn test_context_grouping_single() {
        let nodes = vec![
            make_node_with_context(
                "name-field",
                vec![ContextInfo {
                    name: "user-profile".to_string(),
                    description: Some("User profile section".to_string()),
                }],
                vec![],
            ),
            make_node_with_context(
                "email-field",
                vec![ContextInfo {
                    name: "user-profile".to_string(),
                    description: Some("User profile section".to_string()),
                }],
                vec![],
            ),
        ];

        let (tree, _) = CompactTree::from_tree_nodes(&nodes);

        assert_eq!(tree.tree.len(), 1);
        match &tree.tree[0] {
            TreeEntry::Context { context, description, children } => {
                assert_eq!(context, "user-profile");
                assert_eq!(description.as_deref(), Some("User profile section"));
                assert_eq!(children.len(), 2);
            }
            _ => panic!("Expected Context entry"),
        }
    }

    #[test]
    fn test_context_grouping_nested() {
        let nodes = vec![
            make_node_with_context(
                "save-btn",
                vec![
                    ContextInfo {
                        name: "settings".to_string(),
                        description: Some("Settings page".to_string()),
                    },
                    ContextInfo {
                        name: "form".to_string(),
                        description: None,
                    },
                ],
                vec![],
            ),
        ];

        let (tree, _) = CompactTree::from_tree_nodes(&nodes);

        assert_eq!(tree.tree.len(), 1);
        match &tree.tree[0] {
            TreeEntry::Context { context, description, children } => {
                assert_eq!(context, "settings");
                assert_eq!(description.as_deref(), Some("Settings page"));
                // Should have nested "form" context
                match &children[0] {
                    TreeEntry::Context { context: inner_ctx, children: inner_children, .. } => {
                        assert_eq!(inner_ctx, "form");
                        assert_eq!(inner_children.len(), 1);
                    }
                    _ => panic!("Expected nested Context entry"),
                }
            }
            _ => panic!("Expected Context entry"),
        }
    }

    #[test]
    fn test_context_grouping_mixed() {
        let nodes = vec![
            make_node("header", vec![]),
            make_node_with_context(
                "profile-name",
                vec![ContextInfo {
                    name: "profile".to_string(),
                    description: None,
                }],
                vec![],
            ),
        ];

        let (tree, _) = CompactTree::from_tree_nodes(&nodes);

        // Should have header (no context) and profile context
        assert_eq!(tree.tree.len(), 2);
    }

    #[test]
    fn test_context_printable() {
        let tree = CompactTree {
            schemas: BTreeMap::new(),
            tree: vec![TreeEntry::Context {
                context: "user-profile".to_string(),
                description: Some("User profile section".to_string()),
                children: vec![
                    TreeEntry::Singleton {
                        id: "name-field".to_string(),
                        children: vec![],
                    },
                    TreeEntry::Singleton {
                        id: "email-field".to_string(),
                        children: vec![],
                    },
                ],
            }],
        };

        let output = tree.to_printable();

        let expected = "\
[Tree]
└── [user-profile]: \"User profile section\"
    ├── name-field
    └── email-field
";
        assert_eq!(output, expected);
    }
}
