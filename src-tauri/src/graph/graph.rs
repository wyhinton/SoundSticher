// DAG (Directed Acyclic Graph) and dependencies management

use crate::ops::OpId;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone)]
pub struct OperationGraph {
    // Adjacency list representation of the DAG
    dependencies: HashMap<OpId, Vec<OpId>>, // node -> list of dependencies
    dependents: HashMap<OpId, Vec<OpId>>,   // node -> list of nodes that depend on it
    nodes: HashSet<OpId>,
}

impl OperationGraph {
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
            dependents: HashMap::new(),
            nodes: HashSet::new(),
        }
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, node_id: OpId) {
        self.nodes.insert(node_id);
        self.dependencies.entry(node_id).or_insert_with(Vec::new);
        self.dependents.entry(node_id).or_insert_with(Vec::new);
    }

    /// Add a dependency edge: `dependent` depends on `dependency`
    pub fn add_dependency(&mut self, dependent: OpId, dependency: OpId) -> Result<(), String> {
        // Ensure both nodes exist
        self.add_node(dependent);
        self.add_node(dependency);

        // Check for cycles before adding
        if self.would_create_cycle(dependent, dependency)? {
            return Err(format!("Adding dependency would create a cycle"));
        }

        // Add the dependency
        self.dependencies
            .get_mut(&dependent)
            .unwrap()
            .push(dependency);
        self.dependents
            .get_mut(&dependency)
            .unwrap()
            .push(dependent);

        Ok(())
    }

    /// Remove a node and all its edges
    pub fn remove_node(&mut self, node_id: OpId) {
        if !self.nodes.contains(&node_id) {
            return;
        }

        // Remove from dependencies of other nodes
        if let Some(deps) = self.dependencies.get(&node_id).cloned() {
            for dep in deps {
                if let Some(dependents) = self.dependents.get_mut(&dep) {
                    dependents.retain(|&x| x != node_id);
                }
            }
        }

        // Remove from dependents of other nodes
        if let Some(dependents) = self.dependents.get(&node_id).cloned() {
            for dependent in dependents {
                if let Some(deps) = self.dependencies.get_mut(&dependent) {
                    deps.retain(|&x| x != node_id);
                }
            }
        }

        // Remove the node itself
        self.nodes.remove(&node_id);
        self.dependencies.remove(&node_id);
        self.dependents.remove(&node_id);
    }

    /// Get direct dependencies of a node
    pub fn get_dependencies(&self, node_id: OpId) -> Vec<OpId> {
        self.dependencies.get(&node_id).cloned().unwrap_or_default()
    }

    /// Get direct dependents of a node
    pub fn get_dependents(&self, node_id: OpId) -> Vec<OpId> {
        self.dependents.get(&node_id).cloned().unwrap_or_default()
    }

    /// Get topological ordering of all nodes
    pub fn topological_sort(&self) -> Result<Vec<OpId>, String> {
        let mut in_degree: HashMap<OpId, usize> = HashMap::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        // Calculate in-degrees
        for &node in &self.nodes {
            in_degree.insert(node, self.dependencies.get(&node).unwrap().len());
        }

        // Find nodes with no dependencies
        for (&node, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(node);
            }
        }

        // Process nodes
        while let Some(node) = queue.pop_front() {
            result.push(node);

            // Update in-degrees of dependents
            for &dependent in self.dependents.get(&node).unwrap_or(&Vec::new()) {
                if let Some(degree) = in_degree.get_mut(&dependent) {
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(dependent);
                    }
                }
            }
        }

        if result.len() != self.nodes.len() {
            return Err("Graph contains cycles".to_string());
        }

        Ok(result)
    }

    /// Check if adding an edge would create a cycle
    fn would_create_cycle(&self, from: OpId, to: OpId) -> Result<bool, String> {
        // If 'to' can reach 'from', then adding 'from' -> 'to' would create a cycle
        self.can_reach(to, from)
    }

    /// Check if one node can reach another through the dependency graph
    fn can_reach(&self, start: OpId, target: OpId) -> Result<bool, String> {
        if start == target {
            return Ok(true);
        }

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(start);

        while let Some(node) = queue.pop_front() {
            if visited.contains(&node) {
                continue;
            }
            visited.insert(node);

            if let Some(dependents) = self.dependents.get(&node) {
                for &dependent in dependents {
                    if dependent == target {
                        return Ok(true);
                    }
                    if !visited.contains(&dependent) {
                        queue.push_back(dependent);
                    }
                }
            }
        }

        Ok(false)
    }

    /// Get all nodes that should be invalidated when a node changes
    pub fn get_invalidation_set(&self, changed_node: OpId) -> Vec<OpId> {
        let mut invalidated = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        queue.push_back(changed_node);

        while let Some(node) = queue.pop_front() {
            if visited.contains(&node) {
                continue;
            }
            visited.insert(node);
            invalidated.push(node);

            // Add all dependents to the queue
            if let Some(dependents) = self.dependents.get(&node) {
                for &dependent in dependents {
                    if !visited.contains(&dependent) {
                        queue.push_back(dependent);
                    }
                }
            }
        }

        invalidated
    }
}
