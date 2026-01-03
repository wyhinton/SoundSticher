<script lang="ts">
  import DropDownActionsButton from '$lib/components/DropDownActionsButton.svelte';
  import { type GroupDef, ItemQueryDetailsDictionary } from '$lib/state/groups';

  export let groupName: string;
  export let definition: GroupDef;
  export let isSelected: boolean = false;
  export let resultCount: number | null = null;
  export let onSelect: (groupName: string) => void;
  export let onHover: ((groupName: string) => void) | undefined = undefined;
  export let onHoverLeave: (() => void) | undefined = undefined;
  export let onDelete: ((groupName: string) => void) | undefined = undefined;

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

  // Get icon for query groups
  function getGroupIcon(def: GroupDef): string | null {
    if (def.kind === 'query') {
      return ItemQueryDetailsDictionary[def.query.kind]?.icon || null;
    }
    return null;
  }

  // Create unique dropdown ID for this group item
  const dropdownId = `group-item-${groupName}`;

  // Define dropdown actions
  $: dropdownActions = [
    {
      id: 'rename',
      label: 'Rename Group',
      icon: 'fa-edit',
      disabled: true, // TODO: Implement rename functionality
      onClick: handleRename,
    },
    {
      id: 'delete',
      label: 'Delete Group',
      icon: 'fa-trash',
      variant: 'danger' as const,
      disabled: false,
      onClick: handleDelete,
    },
  ];

  function handleRename(event: MouseEvent) {
    event.stopPropagation();
    // TODO: Implement rename functionality
    console.log('Rename group:', groupName);
  }

  function handleDelete(event: MouseEvent) {
    event.stopPropagation();
    onDelete?.(groupName);
  }
</script>

<div
  class="group-item"
  class:selected={isSelected}
  role="button"
  tabindex="0"
  onclick={() => onSelect(groupName)}
  onkeydown={e => e.key === 'Enter' && onSelect(groupName)}
  onmouseenter={() => onHover?.(groupName)}
  onmouseleave={() => onHoverLeave?.()}
>
  <div class="group-main">
    <div class="group-name-section">
      {#if getGroupIcon(definition)}
        <span class="group-icon">{getGroupIcon(definition)}</span>
      {/if}
      <span class="group-name">{groupName}</span>
    </div>
    <div class="result-count">
      {#if resultCount !== null}
        {resultCount} items
      {:else}
        <em>not evaluated</em>
      {/if}
    </div>
    <div class="group-header-right">
      <span class="group-type">{getGroupType(definition)}</span>
      <DropDownActionsButton
        {dropdownId}
        buttonTitle="Group actions"
        buttonAriaLabel="Group actions for {groupName}"
        actions={dropdownActions}
      />
    </div>
  </div>
  <div class="group-details"></div>
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
  }

  .group-header-right {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .group-name-section {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .group-icon {
    font-size: 12px;
    opacity: 0.8;
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

  .group-details {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .result-count {
    color: #888;
    font-size: 10px;
  }

  /* Ensure dropdown doesn't interfere with group selection */
  .group-header-right :global(.dropdown-container) {
    z-index: 1001;
  }

  .group-header-right :global(.action-button) {
    font-size: 10px;
    padding: 2px 4px;
  }
</style>
