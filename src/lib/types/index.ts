/**
 * Schema-driven operation types and utilities
 *
 * This module re-exports all generated types and provides utilities for
 * working with schema-driven operations.
 */

// Re-export all generated types and constants
export * from './generated/operations';

// Import for utility functions
import {
  type OperationKind,
  type OperationParams,
  type UIControl,
  operationDefaults,
  operationUIControls,
} from './generated/operations';

/**
 * Create a new operation params object with defaults for the given kind
 */
export function createDefaultParams<K extends OperationKind>(kind: K): Record<string, unknown> {
  return { ...operationDefaults[kind] };
}

/**
 * Validate operation params against the schema constraints
 * Returns an array of error messages (empty if valid)
 */
export function validateParams(kind: OperationKind, params: Record<string, unknown>): string[] {
  const errors: string[] = [];
  const controls = operationUIControls[kind] || [];

  for (const control of controls) {
    const value = params[control.key];

    // Check required fields (fields with no default)
    if (value === undefined && control.default === undefined) {
      errors.push(`${control.label} is required`);
      continue;
    }

    // Skip validation if not visible
    if (control.showIf) {
      let visible = true;
      for (const [key, allowedValues] of Object.entries(control.showIf)) {
        const currentValue = params[key] ?? operationDefaults[kind][key];
        if (Array.isArray(allowedValues)) {
          if (!allowedValues.includes(currentValue)) visible = false;
        } else if (currentValue !== allowedValues) {
          visible = false;
        }
      }
      if (!visible) continue;
    }

    const actualValue = value ?? control.default ?? operationDefaults[kind][control.key];

    // Validate ranges for number types
    if ((control.type === 'number' || control.type === 'slider') && actualValue !== undefined) {
      if (typeof actualValue !== 'number') {
        errors.push(`${control.label} must be a number`);
      } else {
        if (control.min !== undefined && actualValue < control.min) {
          errors.push(`${control.label} must be at least ${control.min}`);
        }
        if (control.max !== undefined && actualValue > control.max) {
          errors.push(`${control.label} must be at most ${control.max}`);
        }
      }
    }

    // Validate enum/select values
    if (control.type === 'select' && control.options && actualValue !== undefined) {
      if (!control.options.includes(actualValue as string | number)) {
        errors.push(`${control.label} must be one of: ${control.options.join(', ')}`);
      }
    }
  }

  return errors;
}

/**
 * Get all controls for an operation kind, optionally filtered by visibility
 */
export function getVisibleControls(
  kind: OperationKind,
  params: Record<string, unknown>
): UIControl[] {
  const controls = operationUIControls[kind] || [];

  return controls.filter(control => {
    if (!control.showIf) return true;

    for (const [key, allowedValues] of Object.entries(control.showIf)) {
      const currentValue = params[key] ?? operationDefaults[kind][key];
      if (Array.isArray(allowedValues)) {
        if (!allowedValues.includes(currentValue)) return false;
      } else if (currentValue !== allowedValues) {
        return false;
      }
    }
    return true;
  });
}

/**
 * Merge user params with defaults, ensuring all required fields are present
 */
export function mergeWithDefaults(
  kind: OperationKind,
  params: Partial<Record<string, unknown>>
): Record<string, unknown> {
  return {
    ...operationDefaults[kind],
    ...params,
  };
}

/**
 * Get metadata about an operation kind (for UI display)
 */
export interface OperationMeta {
  kind: OperationKind;
  label: string;
  description: string;
  icon: string;
  category: 'render' | 'edit' | 'meta';
}

export const operationMeta: Record<OperationKind, OperationMeta> = {
  merge: {
    kind: 'merge',
    label: 'Merge',
    description: 'Concatenate multiple audio files into a single output',
    icon: '➕',
    category: 'render',
  },
  sample: {
    kind: 'sample',
    label: 'Sample',
    description: 'Edit audio samples with trim, fade, and gain adjustments',
    icon: '🎵',
    category: 'edit',
  },
  normalize: {
    kind: 'normalize',
    label: 'Normalize',
    description: 'Normalize audio levels to a target loudness',
    icon: '📊',
    category: 'edit',
  },
  export: {
    kind: 'export',
    label: 'Export',
    description: 'Export audio to various formats',
    icon: '💾',
    category: 'render',
  },
  split: {
    kind: 'split',
    label: 'Split',
    description: 'Split audio into multiple segments',
    icon: '✂️',
    category: 'edit',
  },
  pipeline: {
    kind: 'pipeline',
    label: 'Pipeline',
    description: 'Chain multiple operations together',
    icon: '🔀',
    category: 'meta',
  },
};

/**
 * Type guard to check if a string is a valid operation kind
 */
export function isValidOperationKind(kind: string): kind is OperationKind {
  return kind in operationDefaults;
}
