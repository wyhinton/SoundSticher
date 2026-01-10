#!/usr/bin/env node

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

// ES module equivalent of __dirname
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

/**
 * Script to synchronize Tauri command functions with PerformanceState interface
 *
 * This script:
 * 1. Scans all Rust files in src-tauri/src for #[tauri::command] functions
 * 2. Extracts the function names
 * 3. Updates the PerformanceState interface in performance.ts
 * 4. Updates the performanceStore default values
 */

const TAURI_SRC_DIR = path.join(__dirname, '..', 'src-tauri', 'src');
const PERFORMANCE_FILE = path.join(__dirname, '..', 'src', 'lib', 'state', 'performance.ts');

// Function to recursively find all .rs files
function findRustFiles(dir) {
  const files = [];

  function scan(currentDir) {
    const items = fs.readdirSync(currentDir);

    for (const item of items) {
      const itemPath = path.join(currentDir, item);
      const stat = fs.statSync(itemPath);

      if (stat.isDirectory()) {
        // Skip target directory and other build directories
        if (!['target', 'gen', 'capabilities', 'schemas', 'icons'].includes(item)) {
          scan(itemPath);
        }
      } else if (item.endsWith('.rs')) {
        files.push(itemPath);
      }
    }
  }

  scan(dir);
  return files;
}

// Function to extract Tauri command function names from a Rust file
function extractCommandNames(filePath) {
  const content = fs.readFileSync(filePath, 'utf8');
  const lines = content.split('\n');
  const commands = [];

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i].trim();

    // Look for #[tauri::command] annotations
    if (line === '#[tauri::command]' || line.startsWith('#[tauri::command(')) {
      // Look for the function definition in the next few lines
      for (let j = i + 1; j < Math.min(i + 5, lines.length); j++) {
        const nextLine = lines[j].trim();

        // Match function definitions: pub fn function_name( or pub async fn function_name(
        const funcMatch = nextLine.match(/^pub\s+(?:async\s+)?fn\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(/);
        if (funcMatch) {
          const functionName = funcMatch[1];
          commands.push(functionName);
          break;
        }
      }
    }
  }

  return commands;
}

// Function to scan all Rust files and collect command names
function getAllTauriCommands() {
  console.log(`📁 Scanning Rust files in: ${TAURI_SRC_DIR}`);

  const rustFiles = findRustFiles(TAURI_SRC_DIR);
  console.log(`🔍 Found ${rustFiles.length} Rust files`);

  const allCommands = new Set();

  for (const file of rustFiles) {
    const commands = extractCommandNames(file);
    if (commands.length > 0) {
      console.log(`📄 ${path.relative(TAURI_SRC_DIR, file)}: ${commands.join(', ')}`);
      commands.forEach(cmd => allCommands.add(cmd));
    }
  }

  return Array.from(allCommands).sort();
}

// Function to generate the TypeScript interface and default values
function generatePerformanceTypes(commands) {
  const interfaceProperties = commands.map(cmd => `  ${cmd}: PerformanceMetric[];`).join('\n');

  const defaultValues = commands.map(cmd => `  ${cmd}: [],`).join('\n');

  return { interfaceProperties, defaultValues };
}

// Function to update the performance.ts file
function updatePerformanceFile(commands) {
  console.log(`📝 Reading performance file: ${PERFORMANCE_FILE}`);

  if (!fs.existsSync(PERFORMANCE_FILE)) {
    console.error(`❌ Performance file not found: ${PERFORMANCE_FILE}`);
    process.exit(1);
  }

  let content = fs.readFileSync(PERFORMANCE_FILE, 'utf8');
  const { interfaceProperties, defaultValues } = generatePerformanceTypes(commands);

  // Find and replace the PerformanceState interface
  const interfaceRegex = /export interface PerformanceState\s*{[^}]*}/;
  const interfaceReplacement = `export interface PerformanceState {
${interfaceProperties}
}`;

  if (interfaceRegex.test(content)) {
    content = content.replace(interfaceRegex, interfaceReplacement);
    console.log('✅ Updated PerformanceState interface');
  } else {
    console.error('❌ Could not find PerformanceState interface');
    process.exit(1);
  }

  // Find and replace the performanceStore default values
  const storeRegex =
    /export const performanceStore = persisted<PerformanceState>\('performanceState',\s*{[^}]*}\);/;
  const storeReplacement = `export const performanceStore = persisted<PerformanceState>('performanceState', {
${defaultValues}
});`;

  if (storeRegex.test(content)) {
    content = content.replace(storeRegex, storeReplacement);
    console.log('✅ Updated performanceStore default values');
  } else {
    console.error('❌ Could not find performanceStore declaration');
    process.exit(1);
  }

  // Write the updated content back to the file
  fs.writeFileSync(PERFORMANCE_FILE, content, 'utf8');
  console.log(`✅ Successfully updated ${PERFORMANCE_FILE}`);
}

// Function to generate a summary report
function generateReport(commands, existingCommands) {
  console.log('\n📊 SYNC REPORT');
  console.log('================');
  console.log(`🎯 Found ${commands.length} Tauri commands:`);

  // Group commands by categories (based on file patterns)
  const categories = {
    'Audio/Waveform': commands.filter(
      cmd => cmd.includes('sample') || cmd.includes('waveform') || cmd.includes('audio')
    ),
    'Combine/Process': commands.filter(
      cmd => cmd.includes('combine') || cmd.includes('process') || cmd.includes('export')
    ),
    'Timeline/Playback': commands.filter(
      cmd =>
        cmd.includes('timeline') ||
        cmd.includes('play') ||
        cmd.includes('pause') ||
        cmd.includes('stop')
    ),
    'File/IO': commands.filter(
      cmd =>
        cmd.includes('file') ||
        cmd.includes('folder') ||
        cmd.includes('path') ||
        cmd.includes('explorer')
    ),
    'State/Config': commands.filter(
      cmd =>
        cmd.includes('state') ||
        cmd.includes('config') ||
        cmd.includes('setting') ||
        cmd.includes('update')
    ),
    Operations: commands.filter(
      cmd => cmd.includes('operation') || cmd.includes('test') || cmd.includes('scheduler')
    ),
    Other: [],
  };

  // Move uncategorized commands to "Other"
  const categorized = new Set();
  Object.values(categories).forEach(categoryCommands => {
    categoryCommands.forEach(cmd => categorized.add(cmd));
  });

  categories['Other'] = commands.filter(cmd => !categorized.has(cmd));

  // Display categorized commands
  Object.entries(categories).forEach(([category, categoryCommands]) => {
    if (categoryCommands.length > 0) {
      console.log(`\n📁 ${category} (${categoryCommands.length}):`);
      categoryCommands.forEach(cmd => console.log(`   • ${cmd}`));
    }
  });

  // Show changes if we have existing commands to compare
  if (existingCommands && existingCommands.length > 0) {
    const newCommands = commands.filter(cmd => !existingCommands.includes(cmd));
    const removedCommands = existingCommands.filter(cmd => !commands.includes(cmd));

    if (newCommands.length > 0) {
      console.log(`\n➕ New commands (${newCommands.length}):`);
      newCommands.forEach(cmd => console.log(`   • ${cmd}`));
    }

    if (removedCommands.length > 0) {
      console.log(`\n➖ Removed commands (${removedCommands.length}):`);
      removedCommands.forEach(cmd => console.log(`   • ${cmd}`));
    }

    if (newCommands.length === 0 && removedCommands.length === 0) {
      console.log('\n✅ No changes detected');
    }
  }

  console.log('\n🎉 Synchronization complete!');
}

// Function to extract existing commands from performance.ts
function getExistingCommands() {
  try {
    const content = fs.readFileSync(PERFORMANCE_FILE, 'utf8');
    const interfaceMatch = content.match(/export interface PerformanceState\s*{([^}]*)}/);

    if (interfaceMatch) {
      const interfaceBody = interfaceMatch[1];
      const lines = interfaceBody.split('\n');
      const commands = [];

      for (const line of lines) {
        const match = line.trim().match(/^([a-zA-Z_][a-zA-Z0-9_]*)\s*:/);
        if (match) {
          commands.push(match[1]);
        }
      }

      return commands.sort();
    }
  } catch (error) {
    console.log(`⚠️  Could not read existing commands: ${error.message}`);
  }

  return [];
}

// Main execution
function main() {
  console.log('🚀 Starting Tauri Commands → PerformanceState Sync');
  console.log('===================================================');

  try {
    // Get existing commands for comparison
    const existingCommands = getExistingCommands();

    // Scan for all Tauri commands
    const commands = getAllTauriCommands();

    if (commands.length === 0) {
      console.log('⚠️  No Tauri commands found!');
      process.exit(1);
    }

    // Update the performance.ts file
    updatePerformanceFile(commands);

    // Generate report
    generateReport(commands, existingCommands);
  } catch (error) {
    console.error(`❌ Error: ${error.message}`);
    console.error(error.stack);
    process.exit(1);
  }
}

// Run the script
main().catch(error => {
  console.error(`❌ Error: ${error.message}`);
  console.error(error.stack);
  process.exit(1);
});

export { getAllTauriCommands, updatePerformanceFile, generatePerformanceTypes };
