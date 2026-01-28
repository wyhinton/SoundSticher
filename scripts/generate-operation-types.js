/**
 * Generate TypeScript types from JSON Schema operation definitions
 *
 * Usage: node scripts/generate-operation-types.js
 *
 * This script:
 * 1. Reads all operation schemas from /schemas/operations/
 * 2. Generates TypeScript interfaces for each operation
 * 3. Outputs to /src/lib/types/generated/operations.ts
 * 4. Generates Rust enum variants for FrontendOperationDef
 * 5. Outputs to /src-tauri/src/render_ops/generated_operation_defs.rs
 */

import { readFileSync, writeFileSync, readdirSync, existsSync, mkdirSync } from 'fs';
import { join, dirname, basename } from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const SCHEMAS_DIR = join(__dirname, '..', 'schemas', 'operations');
const TS_OUTPUT_DIR = join(__dirname, '..', 'src', 'lib', 'types', 'generated');
const TS_OUTPUT_FILE = join(TS_OUTPUT_DIR, 'operations.ts');
const RUST_OUTPUT_DIR = join(__dirname, '..', 'src-tauri', 'src', 'render_ops');
const RUST_OUTPUT_FILE = join(RUST_OUTPUT_DIR, 'generated_operation_defs.rs');

// Validate all required directories exist
function validateDirectories() {
  const errors = [];

  if (!existsSync(SCHEMAS_DIR)) {
    errors.push(`❌ Schemas directory not found: ${SCHEMAS_DIR}`);
  }

  if (!existsSync(dirname(TS_OUTPUT_DIR))) {
    errors.push(`❌ TypeScript output parent directory not found: ${dirname(TS_OUTPUT_DIR)}`);
  }

  if (!existsSync(dirname(RUST_OUTPUT_DIR))) {
    errors.push(`❌ Rust output parent directory not found: ${dirname(RUST_OUTPUT_DIR)}`);
  }

  if (errors.length > 0) {
    console.error('\n⚠️  CONFIGURATION ERRORS:\n');
    errors.forEach(err => console.error(`  ${err}`));
    console.error('\n💡 Please ensure the following directories exist:');
    console.error(`  - ${SCHEMAS_DIR}`);
    console.error(`  - ${dirname(TS_OUTPUT_DIR)}`);
    console.error(`  - ${dirname(RUST_OUTPUT_DIR)}\n`);
    process.exit(1);
  }
}

// Validate directories before proceeding
validateDirectories();

// Ensure output directories exist
if (!existsSync(TS_OUTPUT_DIR)) {
  mkdirSync(TS_OUTPUT_DIR, { recursive: true });
}

if (!existsSync(RUST_OUTPUT_DIR)) {
  mkdirSync(RUST_OUTPUT_DIR, { recursive: true });
}

/**
 * Convert JSON Schema type to TypeScript type
 */
function schemaTypeToTs(prop) {
  if (prop.const) {
    return JSON.stringify(prop.const);
  }

  if (prop.enum) {
    return prop.enum.map(v => JSON.stringify(v)).join(' | ');
  }

  if (prop.oneOf) {
    return prop.oneOf.map(o => schemaTypeToTs(o)).join(' | ');
  }

  switch (prop.type) {
    case 'string':
      return 'string';
    case 'integer':
    case 'number':
      return 'number';
    case 'boolean':
      return 'boolean';
    case 'array':
      if (prop.items) {
        return `${schemaTypeToTs(prop.items)}[]`;
      }
      return 'unknown[]';
    case 'object':
      if (prop.properties) {
        const props = Object.entries(prop.properties)
          .map(([key, val]) => {
            const optional = !prop.required?.includes(key);
            return `  ${key}${optional ? '?' : ''}: ${schemaTypeToTs(val)};`;
          })
          .join('\n');
        return `{\n${props}\n}`;
      }
      return 'Record<string, unknown>';
    default:
      return 'unknown';
  }
}

/**
 * Extract UI control definitions from schema
 */
function extractUIControls(schema) {
  const controls = [];

  if (schema.properties?.params?.properties) {
    const paramsSchema = schema.properties.params.properties;

    for (const [key, prop] of Object.entries(paramsSchema)) {
      const control = {
        key,
        type:
          prop.ui?.control ??
          (prop.enum ? 'select' : prop.type === 'boolean' ? 'checkbox' : 'text'),
        label: prop.ui?.label ?? key,
        description: prop.description,
        default: prop.default,
        group: prop.ui?.group,
        showIf: prop.ui?.showIf,
      };

      if (prop.enum) {
        control.options = prop.enum;
      }
      if (prop.minimum !== undefined) {
        control.min = prop.minimum;
      }
      if (prop.maximum !== undefined) {
        control.max = prop.maximum;
      }
      if (prop.ui?.step !== undefined) {
        control.step = prop.ui.step;
      }
      if (prop.ui?.placeholder) {
        control.placeholder = prop.ui.placeholder;
      }

      controls.push(control);
    }
  }

  return controls;
}

/**
 * Generate TypeScript interface from schema
 */
function generateInterface(schema, name) {
  const lines = [];

  // Add JSDoc comment
  if (schema.description) {
    lines.push(`/** ${schema.description} */`);
  }

  lines.push(`export interface ${name} {`);

  // Add kind
  if (schema.properties?.kind?.const) {
    lines.push(`  kind: ${JSON.stringify(schema.properties.kind.const)};`);
  }

  // Add other properties
  const skipProps = ['kind', 'params'];
  for (const [key, prop] of Object.entries(schema.properties || {})) {
    if (skipProps.includes(key)) continue;

    // Check if required
    const optional = !schema.required?.includes(key);
    const tsType = schemaTypeToTs(prop);

    if (prop.description) {
      lines.push(`  /** ${prop.description} */`);
    }
    lines.push(`  ${key}${optional ? '?' : ''}: ${tsType};`);
  }

  // Add params if present
  if (schema.properties?.params) {
    const paramsType = schemaTypeToTs(schema.properties.params);
    lines.push(`  /** Operation-specific parameters */`);
    lines.push(`  params: ${paramsType};`);
  }

  lines.push('}');

  return lines.join('\n');
}

/**
 * Convert JSON Schema type to Rust type
 */
function schemaTypeToRust(prop, fieldName) {
  if (prop.const) {
    return 'String'; // Constants are handled by serde
  }

  if (prop.enum) {
    // Enums become String in Rust (validated at runtime)
    return 'String';
  }

  if (prop.$ref === 'operationSource.schema.json') {
    return 'Vec<OperationSource>';
  }

  switch (prop.type) {
    case 'string':
      return 'String';
    case 'integer':
      return 'i64';
    case 'number':
      return 'f64';
    case 'boolean':
      return 'bool';
    case 'array':
      if (prop.items) {
        if (prop.items.$ref === 'operationSource.schema.json') {
          return 'Vec<OperationSource>';
        }
        if (prop.items.type === 'string') {
          return 'Vec<String>';
        }
        if (prop.items.type === 'integer') {
          return 'Vec<i64>';
        }
        if (prop.items.type === 'number') {
          return 'Vec<f64>';
        }
        return 'Vec<serde_json::Value>';
      }
      return 'Vec<serde_json::Value>';
    case 'object':
      // Use serde_json::Value for nested objects or params
      return 'serde_json::Value';
    default:
      return 'serde_json::Value';
  }
}

/**
 * Convert camelCase to snake_case
 */
function camelToSnake(str) {
  return str.replace(/([A-Z])/g, '_$1').toLowerCase();
}

/**
 * Convert snake_case to PascalCase
 */
function snakeToPascal(str) {
  return str
    .split('_')
    .map(s => s.charAt(0).toUpperCase() + s.slice(1))
    .join('');
}

/**
 * Generate Rust enum variant for an operation
 */
function generateRustVariant(kind, schema) {
  const variantName = snakeToPascal(kind);
  const lines = [];

  // Add doc comment
  if (schema.description) {
    lines.push(`    /// ${schema.description}`);
  }

  lines.push(`    #[serde(rename = "${kind}")]`);
  lines.push(`    ${variantName} {`);

  // Always include base fields
  lines.push(`        id: OperationId,`);
  lines.push(`        name: String,`);
  lines.push(`        #[serde(rename = "renderPolicy")]`);
  lines.push(`        render_policy: Option<RenderPolicy>,`);

  // Add operation-specific fields (excluding base fields and params)
  const skipProps = ['id', 'name', 'kind', 'renderPolicy', 'params'];
  for (const [propName, prop] of Object.entries(schema.properties || {})) {
    if (skipProps.includes(propName)) continue;

    const snakeName = camelToSnake(propName);
    const rustType = schemaTypeToRust(prop, propName);
    const isRequired = schema.required?.includes(propName);

    // Add serde rename if needed
    if (snakeName !== propName) {
      lines.push(`        #[serde(rename = "${propName}")]`);
    }

    // Add default for optional fields
    if (!isRequired) {
      lines.push(`        #[serde(default)]`);
    }

    // Wrap in Option if not required
    const finalType = isRequired ? rustType : `Option<${rustType}>`;
    lines.push(`        ${snakeName}: ${finalType},`);
  }

  // Add params field if the schema has params
  if (schema.properties?.params) {
    lines.push(`        #[serde(default)]`);
    lines.push(`        params: Option<serde_json::Value>,`);
  }

  lines.push(`    },`);

  return lines.join('\n');
}

/**
 * Generate the impl methods for the enum
 */
function generateRustImpl(kinds) {
  const lines = [];

  lines.push('impl FrontendOperationDef {');

  // id() method
  lines.push('    pub fn id(&self) -> &OperationId {');
  lines.push('        match self {');
  for (const kind of kinds) {
    lines.push(`            FrontendOperationDef::${snakeToPascal(kind)} { id, .. } => id,`);
  }
  lines.push('        }');
  lines.push('    }');
  lines.push('');

  // name() method
  lines.push('    pub fn name(&self) -> &str {');
  lines.push('        match self {');
  for (const kind of kinds) {
    lines.push(`            FrontendOperationDef::${snakeToPascal(kind)} { name, .. } => name,`);
  }
  lines.push('        }');
  lines.push('    }');
  lines.push('');

  // kind() method
  lines.push('    pub fn kind(&self) -> &str {');
  lines.push('        match self {');
  for (const kind of kinds) {
    lines.push(`            FrontendOperationDef::${snakeToPascal(kind)} { .. } => "${kind}",`);
  }
  lines.push('        }');
  lines.push('    }');
  lines.push('');

  // render_policy() method
  lines.push('    pub fn render_policy(&self) -> Option<&RenderPolicy> {');
  lines.push('        match self {');
  for (const kind of kinds) {
    lines.push(
      `            FrontendOperationDef::${snakeToPascal(kind)} { render_policy, .. } => render_policy.as_ref(),`
    );
  }
  lines.push('        }');
  lines.push('    }');
  lines.push('');

  // sources() method - returns owned Vec for easier use
  lines.push('    pub fn sources(&self) -> Vec<OperationSource> {');
  lines.push('        match self {');
  for (const kind of kinds) {
    lines.push(
      `            FrontendOperationDef::${snakeToPascal(kind)} { sources, .. } => sources.clone(),`
    );
  }
  lines.push('        }');
  lines.push('    }');
  lines.push('');

  // params() method
  lines.push('    pub fn params(&self) -> Option<&serde_json::Value> {');
  lines.push('        match self {');
  for (const kind of kinds) {
    lines.push(
      `            FrontendOperationDef::${snakeToPascal(kind)} { params, .. } => params.as_ref(),`
    );
  }
  lines.push('        }');
  lines.push('    }');

  lines.push('}');

  return lines.join('\n');
}

/**
 * Generate Rust code from schemas
 */
function generateRustCode(schemas) {
  const output = [];

  // Header
  output.push('//! AUTO-GENERATED FILE - DO NOT EDIT');
  output.push('//! Generated from JSON Schemas in /schemas/operations/');
  output.push(`//! Generated at: ${new Date().toISOString()}`);
  output.push('//!');
  output.push('//! This file defines the FrontendOperationDef enum that matches');
  output.push('//! the TypeScript OperationDef type from the frontend.');
  output.push('');
  output.push('use serde::{Deserialize, Serialize};');
  output.push('use std::collections::HashMap;');
  output.push('');
  output.push('// Import types from the render_graph_tests module');
  output.push('// These should be moved to a shared types module eventually');
  output.push(
    'pub use crate::render_ops::render_graph_tests::{OperationId, OperationSource, RenderPolicy};'
  );
  output.push('');

  // Generate the enum
  output.push('/// Operation definition enum matching frontend types');
  output.push('/// Auto-generated from JSON Schemas - DO NOT EDIT MANUALLY');
  output.push('#[derive(Debug, Clone, Serialize, Deserialize)]');
  output.push('#[serde(tag = "kind", rename_all = "lowercase")]');
  output.push('pub enum FrontendOperationDef {');

  const kinds = Object.keys(schemas);
  for (const kind of kinds) {
    output.push(generateRustVariant(kind, schemas[kind]));
  }

  output.push('}');
  output.push('');

  // Generate impl block
  output.push(generateRustImpl(kinds));
  output.push('');

  // Generate OperationsState struct
  output.push('/// Operations state from frontend (matches TypeScript OperationsState)');
  output.push('#[derive(Debug, Clone, Serialize, Deserialize, Default)]');
  output.push('pub struct FrontendOperationsState {');
  output.push('    pub defs: HashMap<OperationId, FrontendOperationDef>,');
  output.push('    #[serde(default)]');
  output.push('    pub order: Vec<OperationId>,');
  output.push('}');
  output.push('');

  // Generate OperationKind enum
  output.push('/// All supported operation kinds');
  output.push('#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]');
  output.push('#[serde(rename_all = "lowercase")]');
  output.push('pub enum OperationKind {');
  for (const kind of kinds) {
    output.push(`    #[serde(rename = "${kind}")]`);
    output.push(`    ${snakeToPascal(kind)},`);
  }
  output.push('}');
  output.push('');

  // Generate impl for OperationKind
  output.push('impl OperationKind {');
  output.push('    pub fn as_str(&self) -> &str {');
  output.push('        match self {');
  for (const kind of kinds) {
    output.push(`            OperationKind::${snakeToPascal(kind)} => "${kind}",`);
  }
  output.push('        }');
  output.push('    }');
  output.push('');
  output.push("    pub fn all() -> &'static [OperationKind] {");
  output.push('        &[');
  for (const kind of kinds) {
    output.push(`            OperationKind::${snakeToPascal(kind)},`);
  }
  output.push('        ]');
  output.push('    }');
  output.push('}');
  output.push('');

  // Generate From<&str> for OperationKind
  output.push('impl std::str::FromStr for OperationKind {');
  output.push('    type Err = String;');
  output.push('');
  output.push('    fn from_str(s: &str) -> Result<Self, Self::Err> {');
  output.push('        match s {');
  for (const kind of kinds) {
    output.push(`            "${kind}" => Ok(OperationKind::${snakeToPascal(kind)}),`);
  }
  output.push('            _ => Err(format!("Unknown operation kind: {}", s)),');
  output.push('        }');
  output.push('    }');
  output.push('}');

  return output.join('\n');
}

/**
 * Main generation function
 */
function generateTypes() {
  console.log('🔧 Generating TypeScript types from JSON Schemas...\n');

  const output = [];
  const schemas = {};
  const uiControls = {};

  // Header
  output.push('/**');
  output.push(' * AUTO-GENERATED FILE - DO NOT EDIT');
  output.push(' * Generated from JSON Schemas in /schemas/operations/');
  output.push(` * Generated at: ${new Date().toISOString()}`);
  output.push(' */');
  output.push('');

  // Read all schema files
  const files = readdirSync(SCHEMAS_DIR).filter(
    f =>
      f.endsWith('.schema.json') &&
      !['index.schema.json', 'baseOperation.schema.json', 'operationSource.schema.json'].includes(f)
  );

  console.log(`📂 Found ${files.length} operation schemas:`);

  for (const file of files) {
    const schemaPath = join(SCHEMAS_DIR, file);
    const schemaContent = readFileSync(schemaPath, 'utf-8');
    const schema = JSON.parse(schemaContent);

    const opKind = basename(file, '.schema.json');
    const interfaceName = `${opKind.charAt(0).toUpperCase()}${opKind.slice(1)}OpParams`;

    console.log(`  📄 ${file} -> ${interfaceName}`);

    schemas[opKind] = schema;

    // Generate interface for params
    if (schema.properties?.params) {
      output.push('');
      output.push(`/** Parameters for ${schema.title || opKind} operation */`);
      output.push(`export interface ${interfaceName} ${schemaTypeToTs(schema.properties.params)}`);
    }

    // Extract UI controls
    uiControls[opKind] = extractUIControls(schema);
  }

  // Generate union type for all params
  output.push('');
  output.push('/** Union of all operation parameter types */');
  output.push(
    `export type OperationParams = ${Object.keys(schemas)
      .map(k => `${k.charAt(0).toUpperCase()}${k.slice(1)}OpParams`)
      .join(' | ')};`
  );

  // Generate operation kinds type
  output.push('');
  output.push('/** All supported operation kinds */');
  output.push(
    `export type OperationKind = ${Object.keys(schemas)
      .map(k => JSON.stringify(k))
      .join(' | ')};`
  );

  // Generate defaults object
  output.push('');
  output.push('/** Default parameter values for each operation type */');
  output.push('export const operationDefaults: Record<OperationKind, Record<string, unknown>> = {');
  for (const [kind, schema] of Object.entries(schemas)) {
    const defaults = {};
    if (schema.properties?.params?.properties) {
      for (const [key, prop] of Object.entries(schema.properties.params.properties)) {
        if (prop.default !== undefined) {
          defaults[key] = prop.default;
        }
      }
    }
    output.push(`  ${kind}: ${JSON.stringify(defaults)},`);
  }
  output.push('};');

  // Generate UI controls configuration
  output.push('');
  output.push('/** UI control configuration for each operation type */');
  output.push('export interface UIControl {');
  output.push('  key: string;');
  output.push('  type: string;');
  output.push('  label: string;');
  output.push('  description?: string;');
  output.push('  default?: unknown;');
  output.push('  group?: string;');
  output.push('  options?: (string | number)[];');
  output.push('  min?: number;');
  output.push('  max?: number;');
  output.push('  step?: number;');
  output.push('  placeholder?: string;');
  output.push('  showIf?: Record<string, unknown>;');
  output.push('}');
  output.push('');
  output.push('export const operationUIControls: Record<OperationKind, UIControl[]> = {');
  for (const [kind, controls] of Object.entries(uiControls)) {
    output.push(
      `  ${kind}: ${JSON.stringify(controls, null, 2)
        .split('\n')
        .map((l, i) => (i === 0 ? l : '  ' + l))
        .join('\n')},`
    );
  }
  output.push('};');

  // Write TypeScript output
  const tsOutputContent = output.join('\n');
  writeFileSync(TS_OUTPUT_FILE, tsOutputContent);

  console.log(`\n✅ Generated ${TS_OUTPUT_FILE}`);
  console.log(`   ${output.length} lines written`);

  // Generate Rust code
  console.log('\n🦀 Generating Rust types from JSON Schemas...\n');
  const rustCode = generateRustCode(schemas);
  writeFileSync(RUST_OUTPUT_FILE, rustCode);

  console.log(`✅ Generated ${RUST_OUTPUT_FILE}`);
  console.log(`   ${rustCode.split('\n').length} lines written`);
}

// Run
generateTypes();
