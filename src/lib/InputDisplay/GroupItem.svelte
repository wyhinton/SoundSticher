<script lang="ts">
  import type { GroupDef, ItemQuery, WhereClause } from '../state/groups';
  import type { AbletonColor } from '../utils/colors';
  import { ABLETON_COLORS } from '../utils/colors';

  export let groupName: string;
  export let definition: GroupDef;
  export let isSelected: boolean = false;
  export let resultCount: number | null = null;
  export let onSelect: (groupName: string) => void;
  export let onUpdateQuery: ((groupName: string, patch: Partial<ItemQuery>) => void) | null = null;

  let expanded = false;

  // Get group type for display
  function getGroupType(def: GroupDef): string {
    switch (def.kind) {
      case 'query':
        return def.query.kind;
      case 'op':
        return `${def.op} (${def.refs.length} refs)`;
      case 'not':
        return `not ${def.ref}`;
      default:
        return 'unknown';
    }
  }

  function handleParamUpdate(patch: Partial<ItemQuery>) {
    if (onUpdateQuery) {
      onUpdateQuery(groupName, patch);
    }
  }

  function toggleExpanded(event: Event) {
    event.stopPropagation();
    expanded = !expanded;
  }
</script>

<div
  class="group-item"
  class:selected={isSelected}
  role="button"
  tabindex="0"
  onclick={() => onSelect(groupName)}
  onkeydown={e => e.key === 'Enter' && onSelect(groupName)}
>
  <div class="group-main">
    <span class="group-name">{groupName}</span>
    <div class="group-header-right">
      <span class="group-type">{getGroupType(definition)}</span>
      {#if definition.kind === 'query' && onUpdateQuery}
        <button class="expand-btn" onclick={toggleExpanded} title="Edit parameters">
          <span class="expand-icon" class:expanded>⚙️</span>
        </button>
      {/if}
    </div>
  </div>
  <div class="group-details">
    <span class="result-count">
      {#if resultCount !== null}
        {resultCount} items
      {:else}
        <em>not evaluated</em>
      {/if}
    </span>
  </div>

  {#if expanded && definition.kind === 'query'}
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <div class="query-params" role="region" onclick={e => e.stopPropagation()}>
      {#if definition.query.kind === 'sectionPercent'}
        <div class="param-row">
          <label for="section-{groupName}">Section Index:</label>
          <input
            id="section-{groupName}"
            type="number"
            min="0"
            step="1"
            value={definition.query.sectionIndex}
            oninput={e =>
              handleParamUpdate({ sectionIndex: parseInt((e.target as HTMLInputElement).value) })}
          />
        </div>
        <div class="param-row">
          <label for="percent-{groupName}">Percent:</label>
          <input
            id="percent-{groupName}"
            type="number"
            min="0"
            max="1"
            step="0.1"
            value={definition.query.percent}
            oninput={e =>
              handleParamUpdate({ percent: parseFloat((e.target as HTMLInputElement).value) })}
          />
        </div>
        <div class="param-row">
          <label for="orderby-{groupName}">Order By:</label>
          <select
            id="orderby-{groupName}"
            value={definition.query.orderBy || 'index'}
            onchange={e =>
              handleParamUpdate({
                orderBy: (e.target as HTMLSelectElement).value as 'index' | 'duration',
              })}
          >
            <option value="index">Index</option>
            <option value="duration">Duration</option>
          </select>
        </div>
        <div class="param-row">
          <label for="take-{groupName}">Take:</label>
          <select
            id="take-{groupName}"
            value={definition.query.take || 'first'}
            onchange={e =>
              handleParamUpdate({
                take: (e.target as HTMLSelectElement).value as 'first' | 'last',
              })}
          >
            <option value="first">First</option>
            <option value="last">Last</option>
          </select>
        </div>
      {:else if definition.query.kind === 'randomSectionPercent'}
        <div class="param-row">
          <label for="rand-section-{groupName}">Section Index:</label>
          <input
            id="rand-section-{groupName}"
            type="number"
            min="0"
            step="1"
            value={definition.query.sectionIndex}
            oninput={e =>
              handleParamUpdate({ sectionIndex: parseInt((e.target as HTMLInputElement).value) })}
          />
        </div>
        <div class="param-row">
          <label for="rand-percent-{groupName}">Percent:</label>
          <input
            id="rand-percent-{groupName}"
            type="number"
            min="0"
            max="1"
            step="0.1"
            value={definition.query.percent}
            oninput={e =>
              handleParamUpdate({ percent: parseFloat((e.target as HTMLInputElement).value) })}
          />
        </div>
        <div class="param-row">
          <label for="seed-{groupName}">Seed:</label>
          <input
            id="seed-{groupName}"
            type="number"
            step="1"
            value={definition.query.seed}
            oninput={e =>
              handleParamUpdate({ seed: parseInt((e.target as HTMLInputElement).value) })}
          />
        </div>
      {:else if definition.query.kind === 'where'}
        <div class="param-row">
          <label for="field-{groupName}">Field:</label>
          <select
            id="field-{groupName}"
            value={definition.query.clause.field}
            onchange={e => {
              const field = (e.target as HTMLSelectElement).value;
              let newClause: WhereClause;
              switch (field) {
                case 'active':
                  newClause = { field: 'active', eq: true };
                  break;
                case 'color':
                  newClause = { field: 'color', eq: ABLETON_COLORS[0]! };
                  break;
                case 'duration':
                  newClause = { field: 'duration', gt: 0 };
                  break;
                case 'path':
                  newClause = { field: 'path', includes: '' };
                  break;
                default:
                  return;
              }
              handleParamUpdate({ clause: newClause });
            }}
          >
            <option value="active">Active</option>
            <option value="color">Color</option>
            <option value="duration">Duration</option>
            <option value="path">Path</option>
          </select>
        </div>

        {#if definition.query.clause.field === 'active'}
          <div class="param-row">
            <label for="active-{groupName}">Active:</label>
            <input
              id="active-{groupName}"
              type="checkbox"
              checked={definition.query.clause.eq}
              onchange={e =>
                handleParamUpdate({
                  clause: { field: 'active', eq: (e.target as HTMLInputElement).checked },
                })}
            />
          </div>
        {:else if definition.query.clause.field === 'color'}
          <div class="param-row">
            <label for="color-{groupName}">Color:</label>
            <select
              id="color-{groupName}"
              value={definition.query.clause.eq.name}
              onchange={e => {
                const selectedName = (e.target as HTMLSelectElement).value;
                const selectedColor = ABLETON_COLORS.find(c => c.name === selectedName);
                if (selectedColor) {
                  handleParamUpdate({ clause: { field: 'color', eq: selectedColor } });
                }
              }}
            >
              {#each ABLETON_COLORS as color}
                <option value={color.name}>{color.name}</option>
              {/each}
            </select>
          </div>
        {:else if definition.query.clause.field === 'duration'}
          <div class="param-row">
            <label for="min-duration-{groupName}">Min Duration:</label>
            <input
              id="min-duration-{groupName}"
              type="number"
              min="0"
              step="0.1"
              value={definition.query.clause.gt || 0}
              oninput={e => {
                const current = (
                  definition.query as { kind: 'where'; clause: WhereClause & { field: 'duration' } }
                ).clause;
                handleParamUpdate({
                  clause: {
                    field: 'duration',
                    gt: parseFloat((e.target as HTMLInputElement).value),
                    lt: current.lt,
                  },
                });
              }}
            />
          </div>
          <div class="param-row">
            <label for="max-duration-{groupName}">Max Duration:</label>
            <input
              id="max-duration-{groupName}"
              type="number"
              min="0"
              step="0.1"
              value={definition.query.clause.lt || ''}
              oninput={e => {
                const current = (
                  definition.query as { kind: 'where'; clause: WhereClause & { field: 'duration' } }
                ).clause;
                const value = (e.target as HTMLInputElement).value;
                handleParamUpdate({
                  clause: {
                    field: 'duration',
                    gt: current.gt,
                    lt: value ? parseFloat(value) : undefined,
                  },
                });
              }}
            />
          </div>
        {:else if definition.query.clause.field === 'path'}
          <div class="param-row">
            <label for="path-{groupName}">Path Contains:</label>
            <input
              id="path-{groupName}"
              type="text"
              value={definition.query.clause.includes}
              oninput={e =>
                handleParamUpdate({
                  clause: { field: 'path', includes: (e.target as HTMLInputElement).value },
                })}
            />
          </div>
        {/if}
      {:else}
        <div class="param-info">No parameters available for this query type</div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .group-item {
    margin: 2px 0;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.2s ease;
    border: 1px solid transparent;
    padding: 4px;
  }

  .group-item:hover {
    background: #2a2a2a;
    border-color: #555;
  }

  .group-item.selected {
    background: #1e40af;
    border-color: #3b82f6;
  }

  .group-main {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 4px;
  }

  .group-header-right {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .group-name {
    font-weight: 500;
    color: #fff;
    font-size: 12px;
  }

  .group-type {
    color: #888;
    font-size: 10px;
    background: #444;
    padding: 2px 6px;
    border-radius: 3px;
  }

  .expand-btn {
    background: none;
    border: none;
    cursor: pointer;
    padding: 2px 4px;
    border-radius: 3px;
    transition: background 0.2s ease;
  }

  .expand-btn:hover {
    background: #555;
  }

  .expand-icon {
    font-size: 10px;
    transition: transform 0.2s ease;
  }

  .expand-icon.expanded {
    transform: rotate(90deg);
  }

  .group-details {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .result-count {
    color: #888;
    font-size: 10px;
  }

  .query-params {
    margin-top: 8px;
    padding: 8px;
    background: #333;
    border-radius: 4px;
    border: 1px solid #555;
  }

  .param-row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 6px;
  }

  .param-row:last-child {
    margin-bottom: 0;
  }

  .param-row label {
    color: #ccc;
    font-size: 10px;
    min-width: 80px;
    flex-shrink: 0;
  }

  .param-row input,
  .param-row select {
    background: #222;
    border: 1px solid #666;
    color: #fff;
    font-size: 10px;
    padding: 2px 4px;
    border-radius: 2px;
    flex: 1;
    min-width: 0;
  }

  .param-row input:focus,
  .param-row select:focus {
    outline: none;
    border-color: #3b82f6;
  }

  .param-row input[type='checkbox'] {
    flex: none;
    width: auto;
  }

  .param-row input[type='number'] {
    max-width: 80px;
  }

  .param-info {
    color: #888;
    font-size: 10px;
    font-style: italic;
    text-align: center;
    padding: 8px;
  }
</style>
