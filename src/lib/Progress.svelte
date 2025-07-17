<script lang="ts">
  export let value: number = 0; // 0 to 100
</script>

<style>
  .progress-container {
    width: 100%;
    height: 6px;
    background-color: #eee;
    overflow: hidden;
  }

  .progress-bar {
    height: 100%;
    background-color: var(--bs-success);
    transition: width 0.3s ease;
    
  }
</style>

<div class="progress-container">
  <div
    class="progress-bar"
    style="width: {Math.min(Math.max(value*100, 0), 100)}%"
  />
</div>
