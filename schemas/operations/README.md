# Operation Schemas

This directory contains JSON Schema definitions for all audio operations in SoundStitch.

## Overview

The schema-driven approach ensures that:

- **Frontend UI** is automatically generated from parameter definitions
- **TypeScript types** are generated at build time
- **Backend validation** uses the same constraints
- **Defaults** are consistent across all layers

## Schema Structure

Each operation schema follows this pattern:

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "operation-name.schema.json",
  "title": "OperationName",
  "description": "Description of what the operation does",
  "type": "object",
  "allOf": [
    { "$ref": "baseOperation.schema.json" }
  ],
  "required": ["kind", "sources", "params"],
  "properties": {
    "kind": {
      "const": "operation-name"
    },
    "sources": {
      "type": "array",
      "items": { "$ref": "operationSource.schema.json" }
    },
    "params": {
      "type": "object",
      "properties": {
        "param_name": {
          "type": "integer",
          "description": "What this parameter does",
          "enum": [value1, value2],
          "default": value1,
          "ui": {
            "control": "select",
            "label": "Human-readable Label",
            "group": "category"
          }
        }
      }
    }
  }
}
```

## UI Hints

The `ui` field is a non-standard extension that controls how the parameter appears in the UI:

| Property      | Description                                                                    |
| ------------- | ------------------------------------------------------------------------------ |
| `control`     | UI control type: `select`, `slider`, `checkbox`, `text`, `number`, `file-path` |
| `label`       | Human-readable label for the control                                           |
| `group`       | Groups related parameters together in the UI                                   |
| `step`        | Step increment for sliders/number inputs                                       |
| `placeholder` | Placeholder text for text inputs                                               |
| `showIf`      | Conditional visibility based on other parameter values                         |

### Conditional Visibility

Use `showIf` to show/hide controls based on other parameter values:

```json
"bit_depth": {
  "type": "integer",
  "ui": {
    "control": "select",
    "showIf": { "format": ["wav", "flac"] }
  }
}
```

This shows `bit_depth` only when `format` is "wav" or "flac".

## Adding a New Operation

1. **Create the schema file**: `schemas/operations/your-operation.schema.json`

2. **Update the index**: Add your schema to `index.schema.json`

3. **Regenerate types**: Run `npm run generate:types`

4. **Use in frontend**: The new operation will automatically get:
   - TypeScript type: `YourOperationOpParams`
   - Default values: `operationDefaults['your-operation']`
   - UI controls: `operationUIControls['your-operation']`

5. **Implement backend**: Add Rust handler for the operation

## Files

| File                          | Purpose                                                   |
| ----------------------------- | --------------------------------------------------------- |
| `baseOperation.schema.json`   | Common fields for all operations (id, name, renderPolicy) |
| `operationSource.schema.json` | Valid input source types (file, group, operation, etc.)   |
| `index.schema.json`           | Lists all operation schemas for tooling                   |
| `merge.schema.json`           | Merge/concatenate audio files                             |
| `sample.schema.json`          | Sample editing (trim, fade, gain)                         |
| `normalize.schema.json`       | Audio normalization                                       |
| `export.schema.json`          | Export to various formats                                 |
| `split.schema.json`           | Split audio into segments                                 |
| `pipeline.schema.json`        | Chain operations together                                 |

## Type Generation

Run `npm run generate:types` to regenerate TypeScript types from schemas.

This outputs:

- `src/lib/types/generated/operations.ts` - All types, defaults, and UI controls

The script runs automatically during `npm run build` and `npm run build:web`.

## Backend Validation

The Rust backend should:

1. Load schemas at startup
2. Validate incoming operation instances
3. Apply defaults for missing parameters

See `src-tauri/src/operations/` for implementation details.
