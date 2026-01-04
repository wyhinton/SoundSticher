// Operation Node and OpId definitions

use slotmap::{DefaultKey, SlotMap};

/// Unique identifier for operations in the graph
pub type OpId = DefaultKey;

/// Operation node in the computation graph
#[derive(Debug, Clone)]
pub struct OperationNode {
    /// Unique identifier for this operation
    pub id: OpId,

    /// Human-readable name for the operation
    pub name: String,

    /// Type of operation (merge, split, pitch, etc.)
    pub operation_type: String,

    /// Operation-specific parameters
    pub parameters: serde_json::Value,

    /// Whether this node's output is currently valid/cached
    pub is_valid: bool,

    /// Timestamp of last modification
    pub last_modified: std::time::SystemTime,
}

impl OperationNode {
    pub fn new(name: String, operation_type: String, parameters: serde_json::Value) -> Self {
        Self {
            id: OpId::default(), // Will be set when inserted into SlotMap
            name,
            operation_type,
            parameters,
            is_valid: false,
            last_modified: std::time::SystemTime::now(),
        }
    }

    /// Mark this node as dirty (needs recomputation)
    pub fn invalidate(&mut self) {
        self.is_valid = false;
        self.last_modified = std::time::SystemTime::now();
    }

    /// Mark this node as valid (output is cached and up-to-date)
    pub fn validate(&mut self) {
        self.is_valid = true;
    }

    /// Update the parameters and invalidate the node
    pub fn update_parameters(&mut self, new_parameters: serde_json::Value) {
        self.parameters = new_parameters;
        self.invalidate();
    }
}

/// Manager for operation nodes using SlotMap for efficient ID management
#[derive(Debug)]
pub struct OperationNodeManager {
    nodes: SlotMap<OpId, OperationNode>,
}

impl OperationNodeManager {
    pub fn new() -> Self {
        Self {
            nodes: SlotMap::new(),
        }
    }

    /// Add a new operation node and return its ID
    pub fn add_node(&mut self, mut node: OperationNode) -> OpId {
        let id = self.nodes.insert(node.clone());
        node.id = id;
        self.nodes[id] = node;
        id
    }

    /// Get a reference to a node by ID
    pub fn get_node(&self, id: OpId) -> Option<&OperationNode> {
        self.nodes.get(id)
    }

    /// Get a mutable reference to a node by ID
    pub fn get_node_mut(&mut self, id: OpId) -> Option<&mut OperationNode> {
        self.nodes.get_mut(id)
    }

    /// Remove a node by ID
    pub fn remove_node(&mut self, id: OpId) -> Option<OperationNode> {
        self.nodes.remove(id)
    }

    /// Get all node IDs
    pub fn get_all_ids(&self) -> Vec<OpId> {
        self.nodes.keys().collect()
    }

    /// Get all nodes
    pub fn get_all_nodes(&self) -> Vec<&OperationNode> {
        self.nodes.values().collect()
    }

    /// Invalidate a node by ID
    pub fn invalidate_node(&mut self, id: OpId) {
        if let Some(node) = self.get_node_mut(id) {
            node.invalidate();
        }
    }

    /// Validate a node by ID
    pub fn validate_node(&mut self, id: OpId) {
        if let Some(node) = self.get_node_mut(id) {
            node.validate();
        }
    }

    /// Update node parameters
    pub fn update_node_parameters(&mut self, id: OpId, parameters: serde_json::Value) {
        if let Some(node) = self.get_node_mut(id) {
            node.update_parameters(parameters);
        }
    }
}
