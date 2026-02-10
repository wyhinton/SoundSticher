<script lang="ts">
  export let title = '';
  export let name = '';
  export let icon = '';
  export let disabled = false;
  export let onClick: () => void;
  export let variant: 'default' | 'primary' | 'secondary' = 'default';
  export let size: 'sm' | 'md' = 'md';
</script>

<button
  class="button-pill"
  class:variant-primary={variant === 'primary'}
  class:variant-secondary={variant === 'secondary'}
  class:size-sm={size === 'sm'}
  onclick={onClick}
  {disabled}
  {title}
>
  <span class="pill-content">
    {#if icon}
      <span class="pill-icon">{icon}</span>
    {/if}
    <span class="pill-name">{name}</span>
    <!-- {#if description}
      <span class="pill-separator">•</span>
      <span class="pill-desc">{description}</span>
    {/if} -->
  </span>
</button>

<style>
  .button-pill {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 2px 4px;
    background: #2a2a2a;
    border: 1px solid #444;
    border-radius: 2px;
    color: #fff;
    cursor: pointer;
    transition: all 0.2s ease;
    text-align: center;
    white-space: nowrap;
    font-size: 11px;
    min-height: 24px;
    margin: 2px 0;
  }

  .button-pill:hover:not(:disabled) {
    background: #3a3a3a;
    border-color: #555;
    transform: translateY(-1px);
  }

  .button-pill:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Variants */
  .button-pill.variant-primary {
    background: #0066cc;
    border-color: #0066cc;
  }

  .button-pill.variant-primary:hover:not(:disabled) {
    background: #0052a3;
    border-color: #0052a3;
  }

  .button-pill.variant-secondary {
    background: #444;
    border-color: #666;
  }

  .button-pill.variant-secondary:hover:not(:disabled) {
    background: #555;
    border-color: #777;
  }

  /* Sizes */
  .button-pill.size-sm {
    padding: 4px 8px;
    font-size: 10px;
    min-height: 20px;
    border-radius: 12px;
  }

  .pill-content {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .pill-icon {
    font-size: inherit;
    line-height: 1;
  }

  .pill-name {
    font-size: inherit;
    font-weight: 500;
    color: inherit;
  }
</style>
