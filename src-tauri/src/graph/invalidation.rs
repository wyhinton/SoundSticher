// Dirty propagation and invalidation system

use crate::graph::{OpId, OperationGraph};
use std::collections::{HashMap, HashSet, VecDeque};

/// Handles invalidation cascading through the operation graph
#[derive(Debug)]
pub struct InvalidationManager {
    /// The operation graph to traverse
    graph: OperationGraph,

    /// Set of nodes that are currently dirty (need recomputation)
    dirty_nodes: HashSet<OpId>,

    /// Priority queue for cooking order (topologically sorted)
    cook_queue: VecDeque<OpId>,
}

impl InvalidationManager {
    pub fn new(graph: OperationGraph) -> Self {
        Self {
            graph,
            dirty_nodes: HashSet::new(),
            cook_queue: VecDeque::new(),
        }
    }

    /// Mark a node as dirty and propagate invalidation
    pub fn invalidate_node(&mut self, node_id: OpId) -> Result<Vec<OpId>, String> {
        // Get all nodes that should be invalidated
        let invalidated = self.graph.get_invalidation_set(node_id);

        // Mark them as dirty
        for &id in &invalidated {
            self.dirty_nodes.insert(id);
        }

        // Rebuild cook queue with topological ordering
        self.rebuild_cook_queue()?;

        Ok(invalidated)
    }

    /// Mark a node as clean (computation complete)
    pub fn validate_node(&mut self, node_id: OpId) {
        self.dirty_nodes.remove(&node_id);
        // Remove from cook queue if present
        let pos = self.cook_queue.iter().position(|&x| x == node_id);
        if let Some(index) = pos {
            self.cook_queue.remove(index);
        }
    }

    /// Get the next node that should be cooked (computed)
    /// Returns nodes in dependency order - dependencies before dependents
    pub fn get_next_cook_node(&mut self) -> Option<OpId> {
        while let Some(node_id) = self.cook_queue.pop_front() {
            if self.dirty_nodes.contains(&node_id) {
                // Check if all dependencies are clean
                let deps = self.graph.get_dependencies(node_id);
                let all_deps_clean = deps.iter().all(|&dep| !self.dirty_nodes.contains(&dep));

                if all_deps_clean {
                    return Some(node_id);
                } else {
                    // Put it back at the end and try again later
                    self.cook_queue.push_back(node_id);
                    continue;
                }
            }
        }
        None
    }

    /// Check if a node is dirty
    pub fn is_dirty(&self, node_id: OpId) -> bool {
        self.dirty_nodes.contains(&node_id)
    }

    /// Get all dirty nodes
    pub fn get_dirty_nodes(&self) -> Vec<OpId> {
        self.dirty_nodes.iter().cloned().collect()
    }

    /// Get the current cook queue
    pub fn get_cook_queue(&self) -> Vec<OpId> {
        self.cook_queue.iter().cloned().collect()
    }

    /// Check if there are any nodes left to cook
    pub fn has_work(&self) -> bool {
        !self.dirty_nodes.is_empty()
    }

    /// Add a new node to the graph
    pub fn add_node(&mut self, node_id: OpId) {
        self.graph.add_node(node_id);
        // New nodes start as dirty
        self.dirty_nodes.insert(node_id);
    }

    /// Add a dependency and handle invalidation
    pub fn add_dependency(&mut self, dependent: OpId, dependency: OpId) -> Result<(), String> {
        self.graph.add_dependency(dependent, dependency)?;

        // If the dependency is dirty, make sure the dependent is also dirty
        if self.dirty_nodes.contains(&dependency) {
            self.invalidate_node(dependent)?;
        }

        Ok(())
    }

    /// Remove a node and clean up
    pub fn remove_node(&mut self, node_id: OpId) -> Result<(), String> {
        // Get dependents before removing
        let dependents = self.graph.get_dependents(node_id);

        // Remove from graph
        self.graph.remove_node(node_id);

        // Clean up dirty state
        self.dirty_nodes.remove(&node_id);

        // Invalidate dependents (they no longer have a valid input)
        for dependent in dependents {
            self.invalidate_node(dependent)?;
        }

        Ok(())
    }

    /// Rebuild the cook queue in topological order
    fn rebuild_cook_queue(&mut self) -> Result<(), String> {
        self.cook_queue.clear();

        // Get topological sort of all dirty nodes
        let all_sorted = self.graph.topological_sort()?;

        // Filter to only include dirty nodes
        for node_id in all_sorted {
            if self.dirty_nodes.contains(&node_id) {
                self.cook_queue.push_back(node_id);
            }
        }

        Ok(())
    }

    /// Update the graph reference (when graph structure changes)
    pub fn update_graph(&mut self, new_graph: OperationGraph) -> Result<(), String> {
        self.graph = new_graph;
        self.rebuild_cook_queue()?;
        Ok(())
    }

    /// Get invalidation statistics
    pub fn get_stats(&self) -> InvalidationStats {
        InvalidationStats {
            total_nodes: self.graph.nodes.len(),
            dirty_nodes: self.dirty_nodes.len(),
            cook_queue_length: self.cook_queue.len(),
        }
    }
}

/// Statistics about the current invalidation state
#[derive(Debug, Clone)]
pub struct InvalidationStats {
    pub total_nodes: usize,
    pub dirty_nodes: usize,
    pub cook_queue_length: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalidation_propagation() {
        let mut graph = OperationGraph::new();
        let node_a = OpId::default();
        let node_b = OpId::default();
        let node_c = OpId::default();

        // Setup: A -> B -> C
        graph.add_node(node_a);
        graph.add_node(node_b);
        graph.add_node(node_c);
        graph.add_dependency(node_b, node_a).unwrap();
        graph.add_dependency(node_c, node_b).unwrap();

        let mut invalidation = InvalidationManager::new(graph);

        // Invalidate A should cascade to B and C
        let invalidated = invalidation.invalidate_node(node_a).unwrap();
        assert_eq!(invalidated.len(), 3); // A, B, C
        assert!(invalidated.contains(&node_a));
        assert!(invalidated.contains(&node_b));
        assert!(invalidated.contains(&node_c));

        // Cook queue should be in order: A, B, C
        let queue = invalidation.get_cook_queue();
        assert_eq!(queue, vec![node_a, node_b, node_c]);
    }
}
