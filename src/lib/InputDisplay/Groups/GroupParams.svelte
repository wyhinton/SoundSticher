<script lang="ts">
  import { type GroupDef, type ItemQuery, type WhereClause } from '$lib/state/groups';
  import { ABLETON_COLORS } from '$lib/utils/colors';

  export let groupName: string;
  export let definition: GroupDef;
  export let onUpdateQuery: (groupName: string, patch: Partial<ItemQuery>) => void;

  function handleParamUpdate(patch: Partial<ItemQuery>) {
    onUpdateQuery(groupName, patch);
  }
</script>

{#if definition.kind === 'query'}
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
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
          oninput={e => handleParamUpdate({ seed: parseInt((e.target as HTMLInputElement).value) })}
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

<style>
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
