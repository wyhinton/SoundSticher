#!/usr/bin/env node

/**
 * Build script to sync logging configuration between Rust backend and TypeScript frontend
 *
 * This script:
 * 1. Parses the Rust LoggingConfig struct to extract field names
 * 2. Generates the corresponding TypeScript LoggingState interface
 * 3. Updates the frontend logging.ts file with the synced interface
 * 4. Ensures consistent naming and types between backend and frontend
 */

import fs from 'fs';
import path from 'path';

const PROJECT_ROOT = path.resolve(process.cwd());
const RUST_LOGGING_PATH = path.join(PROJECT_ROOT, 'src-tauri', 'src', 'logging.rs');
const TS_LOGGING_PATH = path.join(PROJECT_ROOT, 'src', 'lib', 'state', 'logging.ts');

console.log('🔄 Syncing logging configuration between Rust and TypeScript...');

/**
 * Parses Rust LoggingConfig struct to extract field information
 */
function parseRustLoggingConfig(rustContent) {
  // Find the LoggingConfig struct
  const structMatch = rustContent.match(/pub struct LoggingConfig\s*\{([^}]+)\}/s);
  if (!structMatch) {
    throw new Error('Could not find LoggingConfig struct in Rust file');
  }

  const structBody = structMatch[1];

  // Extract field names (excluding console_output which is internal)
  const fieldMatches = structBody.matchAll(/pub\s+(\w+):\s*bool,?/g);
  const fields = [];

  for (const match of fieldMatches) {
    const fieldName = match[1];
    if (fieldName !== 'console_output') {
      fields.push(fieldName);
    }
  }

  return fields;
}

/**
 * Converts Rust snake_case field names to TypeScript camelCase
 */
function rustFieldToTsField(rustField) {
  // Convert snake_case to camelCase
  return rustField.replace(/_([a-z])/g, (_, letter) => letter.toUpperCase()) + 'Log';
}

/**
 * Parses Rust LogSystem enum to extract system names
 */
function parseRustLogSystem(rustContent) {
  // Find the LogSystem enum
  const enumMatch = rustContent.match(/pub enum LogSystem\s*\{([^}]+)\}/s);
  if (!enumMatch) {
    throw new Error('Could not find LogSystem enum in Rust file');
  }

  const enumBody = enumMatch[1];

  // Extract system names (excluding comments and attributes)
  const systemMatches = enumBody.matchAll(/^\s*(\w+),?\s*$/gm);
  const systems = [];

  for (const match of systemMatches) {
    const systemName = match[1];
    systems.push(systemName.toLowerCase());
  }

  return systems;
}

/**
 * Generates TypeScript LoggingState interface
 */
function generateLoggingStateInterface(rustFields, frontendOnlyFields = []) {
  const lines = ['export interface LoggingState {'];

  // Add frontend-only fields first
  if (frontendOnlyFields.length > 0) {
    frontendOnlyFields.forEach(field => {
      lines.push(`  ${field}: boolean;`);
    });
    lines.push('  // Backend logging system toggles');
  }

  // Add backend fields
  rustFields.forEach(rustField => {
    const tsField = rustFieldToTsField(rustField);
    lines.push(`  ${tsField}: boolean;`);
  });

  lines.push('  // Add future logging categories here');
  lines.push('  // performanceLog?: boolean;');
  lines.push('  // audioLog?: boolean;');
  lines.push('  // uiLog?: boolean;');
  lines.push('}');

  return lines.join('\n');
}

/**
 * Generates default LoggingState values
 */
function generateLoggingStateDefault(rustFields, frontendOnlyFields = []) {
  const lines = ["export const loggingState = persisted<LoggingState>('loggingState', {"];

  // Add frontend-only fields
  frontendOnlyFields.forEach(field => {
    lines.push(`  ${field}: false,`);
  });

  // Add backend fields
  rustFields.forEach(rustField => {
    const tsField = rustFieldToTsField(rustField);
    lines.push(`  ${tsField}: false,`);
  });

  lines.push('});');

  return lines.join('\n');
}

/**
 * Generates updateBackendLoggingConfig function
 */
function generateUpdateFunction(rustFields) {
  const lines = [
    '// Update backend logging configuration',
    'export const updateBackendLoggingConfig = async (config: Partial<LoggingState>) => {',
    '  try {',
    '    const backendConfig = {',
  ];

  // Add field mappings
  rustFields.forEach(rustField => {
    const tsField = rustFieldToTsField(rustField);
    lines.push(`      ${rustField}: config.${tsField} ?? false,`);
  });

  lines.push('      console_output: true,');
  lines.push('    };');
  lines.push('');
  lines.push("    await invoke('update_logging_config', { config: backendConfig });");
  lines.push('  } catch (error) {');
  lines.push("    console.error('Failed to update backend logging config:', error);");
  lines.push('  }');
  lines.push('};');

  return lines.join('\n');
}

/**
 * Updates the TypeScript logging file with synced interfaces
 */
function updateTypeScriptFile(rustFields, systems) {
  console.log('📝 Updating TypeScript logging file...');

  const tsContent = fs.readFileSync(TS_LOGGING_PATH, 'utf8');

  // Frontend-only fields that don't have backend equivalents
  const frontendOnlyFields = ['groupsLog', 'selectionLog', 'dragdropLog'];

  // Generate new interface
  const newInterface = generateLoggingStateInterface(rustFields, frontendOnlyFields);

  // Generate new default
  const newDefault = generateLoggingStateDefault(rustFields, frontendOnlyFields);

  // Generate new update function
  const newUpdateFunction = generateUpdateFunction(rustFields);

  // Generate systems union type
  const systemsUnion = systems.map(s => `'${s}'`).join(' | ');

  // Update the file content
  let updatedContent = tsContent;

  // Update LoggingState interface
  updatedContent = updatedContent.replace(/export interface LoggingState \{[^}]+\}/s, newInterface);

  // Update BackendLogMessage system union type
  updatedContent = updatedContent.replace(
    /system: '[^']+' \| '[^']+' \| '[^']+' \| '[^']+' \| '[^']+';/,
    `system: ${systemsUnion};`
  );

  // Update loggingState default
  updatedContent = updatedContent.replace(
    /export const loggingState = persisted<LoggingState>\('loggingState', \{[^}]+\}\);/s,
    newDefault
  );

  // Update updateBackendLoggingConfig function
  updatedContent = updatedContent.replace(
    /\/\/ Update backend logging configuration\nexport const updateBackendLoggingConfig[^}]+\}\;/s,
    newUpdateFunction
  );

  fs.writeFileSync(TS_LOGGING_PATH, updatedContent, 'utf8');

  console.log('✅ TypeScript logging file updated');

  // Log the sync results
  console.log('\n📊 Sync Summary:');
  console.log(`  🦀 Rust fields: ${rustFields.join(', ')}`);
  console.log(`  📝 TypeScript fields: ${rustFields.map(rustFieldToTsField).join(', ')}`);
  console.log(`  🎯 Systems: ${systems.join(', ')}`);
}

/**
 * Main function
 */
function main() {
  try {
    // Check if files exist
    if (!fs.existsSync(RUST_LOGGING_PATH)) {
      throw new Error(`Rust logging file not found: ${RUST_LOGGING_PATH}`);
    }

    if (!fs.existsSync(TS_LOGGING_PATH)) {
      throw new Error(`TypeScript logging file not found: ${TS_LOGGING_PATH}`);
    }

    // Parse Rust file
    console.log('🦀 Parsing Rust logging configuration...');
    const rustContent = fs.readFileSync(RUST_LOGGING_PATH, 'utf8');
    const rustFields = parseRustLoggingConfig(rustContent);
    const systems = parseRustLogSystem(rustContent);

    console.log(`Found ${rustFields.length} backend logging fields and ${systems.length} systems`);

    // Update TypeScript file
    updateTypeScriptFile(rustFields, systems);

    console.log('\n🎉 Logging configuration sync completed successfully!');
  } catch (error) {
    console.error('\n❌ Error syncing logging configuration:', error.message);
    process.exit(1);
  }
}

// Run if this script is executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  main();
}

export { main as syncLoggingConfig };
