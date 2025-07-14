<script lang="ts">
  const durationSeconds = 180; // 3 minutes
  const interval = 10; // 10s per major tick
</script>

<style>
  .ruler-container {
    display: flex;
    overflow-x: scroll;
    white-space: nowrap;
    background-color: #111;
    color: #999;
    font-family: sans-serif;
    height: 30px;
    padding: 0 10px;
  }

  .tick {
    display: inline-block;
    width: 80px;
    text-align: center;
    position: relative;
    font-size: 0.75rem;
  }

  .tick::before {
    content: "";
    position: absolute;
    top: 0;
    left: 50%;
    width: 1px;
    height: 8px;
    background: #666;
    transform: translateX(-50%);
  }
</style>

<div class="ruler-container">
  {#each Array.from({ length: Math.floor(durationSeconds / interval) + 1 }, (_, i) => i) as i}
    <div class="tick">{Math.floor((i * interval) / 60)}:{String((i * interval) % 60).padStart(2, '0')}</div>
  {/each}
</div>
