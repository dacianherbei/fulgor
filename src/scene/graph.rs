use std::fmt;
use crate::numerics;
/// Scene graph and node definitions for the `fuller` library.
///
/// Node stores children as `Vec<Box<Node<T>>>` to avoid recursive-size issues.
/// Nodes are generic over the numeric precision `T` so they can carry templated primitives.

use crate::scene::primitive::Splat;

/// A scene graph root container.
#[derive(Debug, Clone)]
pub struct SceneGraph<T: numerics::types::traits::FloatingPoint = f32> {
    pub name: String,
    pub root: Node<T>,
}

impl<T: numerics::types::traits::FloatingPoint> SceneGraph<T> {
    /// Create a new scene graph with a root node.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            root: Node::new("root"),
        }
    }

    /// Add a child node to the root.
    pub fn add_node_to_root(&mut self, node: Node<T>) {
        self.root.children.push(Box::new(node));
    }

    /// Traverse the scene graph, calling `f` for each node (pre-order).
    pub fn traverse<F: FnMut(&Node<T>)>(&self, mut f: F) {
        self.root.traverse(&mut f);
    }

    /// Mutably traverse the scene graph, calling `f` for each node (pre-order).
    pub fn traverse_mut<F: FnMut(&mut Node<T>)>(&mut self, mut f: F) {
        self.root.traverse_mut(&mut f);
    }

    /// Find a node by name (first match, pre-order). Returns a reference if found.
    pub fn find_node_by_name(&self, name: &str) -> Option<&Node<T>> {
        self.root.find_by_name(name)
    }

    /// Find a mutable node by name (first match, pre-order). Returns a mutable reference if found.
    pub fn find_node_by_name_mut(&mut self, name: &str) -> Option<&mut Node<T>> {
        self.root.find_by_name_mut(name)
    }
}

/// A node in the scene graph.
///
/// Minimal fields:
/// - name: `String`
/// - children: `Vec<Box<Node<T>>>`
/// - primitives: a collection of splats attached to the node (optional)
#[derive(Clone)]
pub struct Node<T: numerics::types::traits::FloatingPoint = f32> {
    pub name: String,
    pub children: Vec<Box<Node<T>>>,
    pub splats: Vec<Splat<T>>,
}

impl<T: numerics::types::traits::FloatingPoint> Node<T> {
    /// Create a new node with given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            children: Vec::new(),
            splats: Vec::new(),
        }
    }

    /// Convenience: create an empty named node and push it as a child.
    pub fn add_child(&mut self, child: Node<T>) {
        self.children.push(Box::new(child));
    }

    /// Attach a splat primitive to the node.
    pub fn attach_splat(&mut self, splat: Splat<T>) {
        self.splats.push(splat);
    }

    /// Pre-order traversal (immutable)
    pub fn traverse<F: FnMut(&Node<T>)>(&self, f: &mut F) {
        f(self);
        for child in &self.children {
            child.traverse(f);
        }
    }

    /// Pre-order traversal (mutable)
    pub fn traverse_mut<F: FnMut(&mut Node<T>)>(&mut self, f: &mut F) {
        f(self);
        for child in &mut self.children {
            child.traverse_mut(f);
        }
    }

    /// Find first node by name (immutable).
    pub fn find_by_name(&self, target: &str) -> Option<&Node<T>> {
        if self.name == target {
            return Some(self);
        }
        for child in &self.children {
            if let Some(found) = child.find_by_name(target) {
                return Some(found);
            }
        }
        None
    }

    /// Find first node by name (mutable).
    pub fn find_by_name_mut(&mut self, target: &str) -> Option<&mut Node<T>> {
        if self.name == target {
            return Some(self);
        }
        for child in &mut self.children {
            if let Some(found) = child.find_by_name_mut(target) {
                return Some(found);
            }
        }
        None
    }
}

impl<T: numerics::types::traits::FloatingPoint> fmt::Debug for Node<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Keep debug concise: name and counts
        f.debug_struct("Node")
            .field("name", &self.name)
            .field("children_count", &self.children.len())
            .field("splats_count", &self.splats.len())
            .finish()
    }
}

impl<T: numerics::types::traits::FloatingPoint> fmt::Display for Node<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Node(\"{}\", children={}, splats={})", self.name, self.children.len(), self.splats.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenegraph_basic_operations() {
        let mut g: SceneGraph<f32> = SceneGraph::new("test");
        let mut n = Node::new("child1");
        let splat = Splat::new([0.0, 1.0, 2.0], 0.5, [1.0, 0.0, 0.0, 1.0]);
        n.attach_splat(splat);
        g.add_node_to_root(n);

        // traverse and collect names
        let mut names = Vec::new();
        g.traverse(|node| names.push(node.name.clone()));
        assert!(names.contains(&"root".to_string()));
        assert!(names.contains(&"child1".to_string()));

        // find node
        let found = g.find_node_by_name("child1");
        assert!(found.is_some());
        assert_eq!(found.unwrap().splats.len(), 1);
    }
}
