#!/usr/bin/env node

/**
 * Extract Tauri command line numbers and locations
 * Generates a TypeScript object mapping command names to their file locations and line numbers
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const TAURI_SRC_DIR = path.join(__dirname, '../src-tauri/src');
const OUTPUT_FILE = path.join(__dirname, '../src/lib/generated/tauri_commands.ts');

/**
 * Recursively find all .rs files in a directory
 */
function findRustFiles(dir, files = []) {
  const entries = fs.readdirSync(dir, { withFileTypes: true });

  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      findRustFiles(fullPath, files);
    } else if (entry.isFile() && entry.name.endsWith('.rs')) {
      files.push(fullPath);
    }
  }

  return files;
}

/**
 * Extract command info from a Rust file
 */
function extractCommandsFromFile(filePath) {
  const content = fs.readFileSync(filePath, 'utf-8');
  const lines = content.split('\n');
  const commands = [];

  // Regex to match #[tauri::command]
  const commandRegex = /#\[tauri::command\]/;
  const functionRegex = /^\s*(?:async\s+)?pub\s+(?:async\s+)?fn\s+(\w+)\s*\(/;

  for (let i = 0; i < lines.length; i++) {
    if (commandRegex.test(lines[i])) {
      // Look at the next few lines for the function definition
      // Search up to 10 lines ahead to handle multi-line attributes
      for (let j = i + 1; j < Math.min(i + 10, lines.length); j++) {
        const match = lines[j].match(functionRegex);
        if (match) {
          const commandName = match[1];
          const lineNumber = j + 1; // Convert to 1-based line numbering
          commands.push({
            name: commandName,
            line_number: lineNumber,
            file_path: filePath,
            file_name: path.basename(filePath),
          });
          break;
        }
      }
    }
  }

  return commands;
}

/**
 * Generate TypeScript file content
 */
function generateTypeScriptObject(commands) {
  // Sort commands by name
  const sortedCommands = commands.sort((a, b) => a.name.localeCompare(b.name));

  // Create the TypeScript object
  const commandsObj = {};
  for (const cmd of sortedCommands) {
    commandsObj[cmd.name] = {
      line_number: cmd.line_number,
      file_path: cmd.file_path,
      file_name: cmd.file_name,
    };
  }

  // Generate TypeScript content
  const tsContent = `// Auto-generated file - Do not edit manually
// Run: node scripts/extract_command_line_numbers.js

export interface CommandInfo {
  line_number: number;
  file_path: string;
  file_name: string;
}

export interface CommandMap {
  [commandName: string]: CommandInfo;
}

export const TAURI_COMMANDS: CommandMap = ${JSON.stringify(commandsObj, null, 2)};

export function getCommandInfo(commandName: string): CommandInfo | undefined {
  return TAURI_COMMANDS[commandName];
}

export function getCommandLineNumber(commandName: string): number | undefined {
  return TAURI_COMMANDS[commandName]?.line_number;
}

export function getCommandFilePath(commandName: string): string | undefined {
  return TAURI_COMMANDS[commandName]?.file_path;
}
`;

  return tsContent;
}

/**
 * Main execution
 */
function main() {
  try {
    console.log(`📂 Scanning Rust files in: ${TAURI_SRC_DIR}`);

    // Find all Rust files
    const rustFiles = findRustFiles(TAURI_SRC_DIR);
    console.log(`✅ Found ${rustFiles.length} Rust files`);

    // Extract commands from each file
    let allCommands = [];
    for (const file of rustFiles) {
      const commands = extractCommandsFromFile(file);
      if (commands.length > 0) {
        console.log(`  📄 ${path.relative(TAURI_SRC_DIR, file)}: ${commands.length} command(s)`);
        allCommands = allCommands.concat(commands);
      }
    }

    console.log(`\n🔍 Extracted ${allCommands.length} total commands`);

    // Generate TypeScript file
    const tsContent = generateTypeScriptObject(allCommands);

    // Ensure output directory exists
    const outputDir = path.dirname(OUTPUT_FILE);
    if (!fs.existsSync(outputDir)) {
      fs.mkdirSync(outputDir, { recursive: true });
      console.log(`📁 Created directory: ${outputDir}`);
    }

    // Write TypeScript file
    fs.writeFileSync(OUTPUT_FILE, tsContent, 'utf-8');
    console.log(`\n✨ Generated: ${OUTPUT_FILE}`);

    // Print summary
    console.log(`\n📊 Command Summary:`);
    allCommands.sort((a, b) => a.name.localeCompare(b.name)).forEach((cmd) => {
      console.log(`  - ${cmd.name} (${cmd.file_name}:${cmd.line_number})`);
    });

    console.log(`\n✅ Done!`);
  } catch (error) {
    console.error('❌ Error:', error);
    process.exit(1);
  }
}

main();
