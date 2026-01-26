/**
 * Generate TypeScript types from JSON Schema operation definitions
 *
 * Usage: node scripts/generate-operation-types.js
 *
 * This script:
 * 1. Reads all operation schemas from /schemas/operations/
 * 2. Generates TypeScript interfaces for each operation
 * 3. Outputs to /src/lib/types/generated/operations.ts
 */

import { readFileSync, writeFileSync, readdirSync, existsSync, mkdirSync } from 'fs';
import { join, dirname, basename } from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const SCHEMAS_DIR = join(__dirname, '..', 'schemas', 'operations');
const OUTPUT_DIR = join(__dirname, '..', 'src', 'lib', 'types', 'generated');
const OUTPUT_FILE = join(OUTPUT_DIR, 'operations.ts');

// Ensure output directory exists
if (!existsSync(OUTPUT_DIR)) {
  mkdirSync(OUTPUT_DIR, { recursive: true });
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

  // Write output
  const outputContent = output.join('\n');
  writeFileSync(OUTPUT_FILE, outputContent);

  console.log(`\n✅ Generated ${OUTPUT_FILE}`);
  console.log(`   ${output.length} lines written`);
}

// Run
generateTypes();
