<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { OperationKind, UIControl } from '$lib/types/generated/operations';
  import { operationUIControls, operationDefaults } from '$lib/types/generated/operations';

  // Props
  export let operationKind: OperationKind;
  export let params: Record<string, unknown> = {};

  const dispatch = createEventDispatcher<{
    change: { key: string; value: unknown };
    paramsChange: Record<string, unknown>;
  }>();

  // Get controls for this operation kind
  $: controls = operationUIControls[operationKind] || [];

  // Group controls by their group property
  $: groupedControls = controls.reduce(
    (acc, control) => {
      const group = control.group || 'general';
      if (!acc[group]) acc[group] = [];
      acc[group].push(control);
      return acc;
    },
    {} as Record<string, UIControl[]>
  );

  // Check if a control should be visible based on showIf conditions
  function isControlVisible(control: UIControl): boolean {
    if (!control.showIf) return true;

    for (const [key, allowedValues] of Object.entries(control.showIf)) {
      const currentValue = params[key] ?? operationDefaults[operationKind][key];
      if (Array.isArray(allowedValues)) {
        if (!allowedValues.includes(currentValue)) return false;
      } else if (currentValue !== allowedValues) {
        return false;
      }
    }
    return true;
  }

  // Get current value for a control
  function getValue(control: UIControl): unknown {
    return params[control.key] ?? control.default ?? operationDefaults[operationKind][control.key];
  }

  // Handle value change
  function handleChange(control: UIControl, event: Event) {
    const target = event.target as HTMLInputElement | HTMLSelectElement;
    let value: unknown;

    switch (control.type) {
      case 'checkbox':
        value = (target as HTMLInputElement).checked;
        break;
      case 'number':
      case 'slider':
        value = parseFloat(target.value);
        break;
      case 'select':
        // Check if the options are numbers
        if (control.options?.every(opt => typeof opt === 'number')) {
          value = parseFloat(target.value);
        } else {
          value = target.value;
        }
        break;
      default:
        value = target.value;
    }

    // Update local params
    params = { ...params, [control.key]: value };

    // Dispatch events
    dispatch('change', { key: control.key, value });
    dispatch('paramsChange', params);
  }

  // Format group name for display
  function formatGroupName(name: string): string {
    return name.replace(/_/g, ' ').replace(/\b\w/g, c => c.toUpperCase());
  }
</script>

<div class="params-form">
  {#each Object.entries(groupedControls) as [groupName, groupControls]}
    <fieldset class="param-group">
      <legend>{formatGroupName(groupName)}</legend>

      {#each groupControls as control}
        {#if isControlVisible(control)}
          <div class="param-row">
            <label for={`param-${control.key}`} class="param-label">
              {control.label}
              {#if control.description}
                <span class="param-hint" title={control.description}>?</span>
              {/if}
            </label>

            <div class="param-control">
              {#if control.type === 'select' && control.options}
                <select
                  id={`param-${control.key}`}
                  value={getValue(control)}
                  on:change={e => handleChange(control, e)}
                >
                  {#each control.options as option}
                    <option value={option}>{option}</option>
                  {/each}
                </select>
              {:else if control.type === 'checkbox'}
                <input
                  id={`param-${control.key}`}
                  type="checkbox"
                  checked={getValue(control) === true}
                  on:change={e => handleChange(control, e)}
                />
              {:else if control.type === 'slider'}
                <div class="slider-control">
                  <input
                    id={`param-${control.key}`}
                    type="range"
                    min={control.min ?? 0}
                    max={control.max ?? 100}
                    step={control.step ?? 1}
                    value={getValue(control)}
                    on:input={e => handleChange(control, e)}
                  />
                  <span class="slider-value">{getValue(control)}</span>
                </div>
              {:else if control.type === 'number'}
                <input
                  id={`param-${control.key}`}
                  type="number"
                  min={control.min}
                  max={control.max}
                  step={control.step ?? 1}
                  value={getValue(control)}
                  on:input={e => handleChange(control, e)}
                />
              {:else if control.type === 'file-path'}
                <div class="file-path-control">
                  <input
                    id={`param-${control.key}`}
                    type="text"
                    value={getValue(control)}
                    placeholder={control.placeholder}
                    on:input={e => handleChange(control, e)}
                  />
                  <button type="button" class="browse-btn">Browse</button>
                </div>
              {:else}
                <input
                  id={`param-${control.key}`}
                  type="text"
                  value={getValue(control)}
                  placeholder={control.placeholder}
                  on:input={e => handleChange(control, e)}
                />
              {/if}
            </div>
          </div>
        {/if}
      {/each}
    </fieldset>
  {/each}
</div>

<style>
  .params-form {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    font-size: 0.875rem;
  }

  .param-group {
    border: 1px solid var(--border-color, #333);
    border-radius: 6px;
    padding: 0.75rem;
    margin: 0;
    background: var(--bg-secondary, #1e1e1e);
  }

  .param-group legend {
    padding: 0 0.5rem;
    font-weight: 600;
    color: var(--text-primary, #fff);
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .param-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.5rem 0;
    border-bottom: 1px solid var(--border-color-subtle, #2a2a2a);
  }

  .param-row:last-child {
    border-bottom: none;
  }

  .param-label {
    flex: 1;
    color: var(--text-secondary, #ccc);
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .param-hint {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--accent-color, #4a9eff);
    color: white;
    font-size: 0.7rem;
    cursor: help;
  }

  .param-control {
    flex: 1;
    max-width: 200px;
  }

  .param-control select,
  .param-control input[type='text'],
  .param-control input[type='number'] {
    width: 100%;
    padding: 0.4rem 0.6rem;
    border: 1px solid var(--border-color, #444);
    border-radius: 4px;
    background: var(--bg-input, #2a2a2a);
    color: var(--text-primary, #fff);
    font-size: 0.85rem;
  }

  .param-control select:focus,
  .param-control input:focus {
    outline: none;
    border-color: var(--accent-color, #4a9eff);
  }

  .param-control input[type='checkbox'] {
    width: 18px;
    height: 18px;
    cursor: pointer;
  }

  .slider-control {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .slider-control input[type='range'] {
    flex: 1;
    cursor: pointer;
  }

  .slider-value {
    min-width: 3rem;
    text-align: right;
    color: var(--text-secondary, #ccc);
    font-family: monospace;
    font-size: 0.8rem;
  }

  .file-path-control {
    display: flex;
    gap: 0.5rem;
  }

  .file-path-control input {
    flex: 1;
  }

  .browse-btn {
    padding: 0.4rem 0.8rem;
    border: 1px solid var(--border-color, #444);
    border-radius: 4px;
    background: var(--bg-button, #3a3a3a);
    color: var(--text-primary, #fff);
    cursor: pointer;
    font-size: 0.8rem;
  }

  .browse-btn:hover {
    background: var(--bg-button-hover, #4a4a4a);
  }
</style>
