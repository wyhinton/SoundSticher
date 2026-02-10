//! AUTO-GENERATED FILE - DO NOT EDIT
//! Generated from JSON Schemas in /schemas/operations/
//! Generated at: 2026-01-26T21:48:31.208Z
//!
//! This file defines the FrontendOperationDef enum that matches
//! the TypeScript OperationDef type from the frontend.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Import types from the render_graph_tests module
// These should be moved to a shared types module eventually
pub use crate::render_ops::render_graph_tests::{OperationId, OperationSource, RenderPolicy};

/// Operation definition enum matching frontend types
/// Auto-generated from JSON Schemas - DO NOT EDIT MANUALLY
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum FrontendOperationDef {
    /// Exports audio to a specific format and location
    #[serde(rename = "export")]
    Export {
        id: OperationId,
        name: String,
        #[serde(rename = "renderPolicy")]
        render_policy: Option<RenderPolicy>,
        sources: Vec<OperationSource>,
        #[serde(rename = "outputPath")]
        output_path: String,
        #[serde(default)]
        params: Option<serde_json::Value>,
    },
    /// Concatenates multiple audio files into a single output file
    #[serde(rename = "merge")]
    Merge {
        id: OperationId,
        name: String,
        #[serde(rename = "renderPolicy")]
        render_policy: Option<RenderPolicy>,
        sources: Vec<OperationSource>,
        #[serde(rename = "outputPath")]
        output_path: String,
        #[serde(default)]
        params: Option<serde_json::Value>,
    },
    /// Chains multiple operations together in sequence
    #[serde(rename = "pipeline")]
    Pipeline {
        id: OperationId,
        name: String,
        #[serde(rename = "renderPolicy")]
        render_policy: Option<RenderPolicy>,
        sources: Vec<OperationSource>,
        operations: Vec<String>,
        #[serde(default)]
        params: Option<serde_json::Value>,
    },
    /// Loads and processes individual audio samples
    #[serde(rename = "sample")]
    Sample {
        id: OperationId,
        name: String,
        #[serde(rename = "renderPolicy")]
        render_policy: Option<RenderPolicy>,
        sources: Vec<OperationSource>,
        #[serde(default)]
        params: Option<serde_json::Value>,
    },
}

impl FrontendOperationDef {
    pub fn id(&self) -> &OperationId {
        match self {
            FrontendOperationDef::Export { id, .. } => id,
            FrontendOperationDef::Merge { id, .. } => id,
            FrontendOperationDef::Pipeline { id, .. } => id,
            FrontendOperationDef::Sample { id, .. } => id,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            FrontendOperationDef::Export { name, .. } => name,
            FrontendOperationDef::Merge { name, .. } => name,
            FrontendOperationDef::Pipeline { name, .. } => name,
            FrontendOperationDef::Sample { name, .. } => name,
        }
    }

    pub fn kind(&self) -> &str {
        match self {
            FrontendOperationDef::Export { .. } => "export",
            FrontendOperationDef::Merge { .. } => "merge",
            FrontendOperationDef::Pipeline { .. } => "pipeline",
            FrontendOperationDef::Sample { .. } => "sample",
        }
    }

    pub fn render_policy(&self) -> Option<&RenderPolicy> {
        match self {
            FrontendOperationDef::Export { render_policy, .. } => render_policy.as_ref(),
            FrontendOperationDef::Merge { render_policy, .. } => render_policy.as_ref(),
            FrontendOperationDef::Pipeline { render_policy, .. } => render_policy.as_ref(),
            FrontendOperationDef::Sample { render_policy, .. } => render_policy.as_ref(),
        }
    }

    pub fn sources(&self) -> Vec<OperationSource> {
        match self {
            FrontendOperationDef::Export { sources, .. } => sources.clone(),
            FrontendOperationDef::Merge { sources, .. } => sources.clone(),
            FrontendOperationDef::Pipeline { sources, .. } => sources.clone(),
            FrontendOperationDef::Sample { sources, .. } => sources.clone(),
        }
    }

    pub fn params(&self) -> Option<&serde_json::Value> {
        match self {
            FrontendOperationDef::Export { params, .. } => params.as_ref(),
            FrontendOperationDef::Merge { params, .. } => params.as_ref(),
            FrontendOperationDef::Pipeline { params, .. } => params.as_ref(),
            FrontendOperationDef::Sample { params, .. } => params.as_ref(),
        }
    }
}

/// Operations state from frontend (matches TypeScript OperationsState)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FrontendOperationsState {
    pub defs: HashMap<OperationId, FrontendOperationDef>,
    #[serde(default)]
    pub order: Vec<OperationId>,
}

/// All supported operation kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OperationKind {
    #[serde(rename = "export")]
    Export,
    #[serde(rename = "merge")]
    Merge,
    #[serde(rename = "pipeline")]
    Pipeline,
    #[serde(rename = "sample")]
    Sample,
}

impl OperationKind {
    pub fn as_str(&self) -> &str {
        match self {
            OperationKind::Export => "export",
            OperationKind::Merge => "merge",
            OperationKind::Pipeline => "pipeline",
            OperationKind::Sample => "sample",
        }
    }

    pub fn all() -> &'static [OperationKind] {
        &[
            OperationKind::Export,
            OperationKind::Merge,
            OperationKind::Pipeline,
            OperationKind::Sample,
        ]
    }
}

impl std::str::FromStr for OperationKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "export" => Ok(OperationKind::Export),
            "merge" => Ok(OperationKind::Merge),
            "pipeline" => Ok(OperationKind::Pipeline),
            "sample" => Ok(OperationKind::Sample),
            _ => Err(format!("Unknown operation kind: {}", s)),
        }
    }
}
