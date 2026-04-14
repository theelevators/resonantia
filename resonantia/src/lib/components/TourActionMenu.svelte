<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let open = false;

  const dispatch = createEventDispatcher<{
    toggle: void;
    refresh: void;
    demo: void;
    checkin: void;
    settings: void;
  }>();
</script>

<div class="menu-wrap">
  <button
    class="nav-btn menu-btn"
    data-tour-target="menu-toggle"
    class:open={open}
    on:click={() => dispatch('toggle')}
    aria-label="Open menu"
    aria-expanded={open}
  >
    ☰
  </button>
  {#if open}
    <div class="menu-popover" role="menu" aria-label="Weaver actions">
      <button class="menu-item" on:click={() => dispatch('refresh')}>refresh view</button>
      <button class="menu-item" on:click={() => dispatch('demo')}>run cinematic demo</button>
      <button class="menu-item" data-tour-target="checkin" on:click={() => dispatch('checkin')}>check in</button>
      <button class="menu-item" data-tour-target="settings" on:click={() => dispatch('settings')}>settings</button>
    </div>
  {/if}
</div>

<style>
  .nav-btn {
    font-family: 'Departure Mono', monospace;
    font-size: 10px;
    letter-spacing: 0.08em;
    color: rgba(255, 255, 255, 0.3);
    background: transparent;
    border: 0.5px solid rgba(255, 255, 255, 0.08);
    border-radius: 4px;
    padding: 4px 10px;
    cursor: pointer;
    transition: color 0.2s, border-color 0.2s;
  }

  .nav-btn:hover {
    color: rgba(255, 255, 255, 0.75);
    border-color: rgba(255, 255, 255, 0.2);
  }

  .menu-wrap {
    position: relative;
  }

  .menu-btn {
    min-width: 36px;
    padding: 4px 0;
    text-align: center;
    font-size: 13px;
    line-height: 1;
  }

  .menu-btn.open {
    color: rgba(255, 255, 255, 0.72);
    border-color: rgba(255, 255, 255, 0.22);
  }

  .menu-popover {
    position: absolute;
    top: calc(100% + 8px);
    right: 0;
    width: 148px;
    padding: 6px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    background: rgba(10, 11, 14, 0.97);
    border: 0.5px solid rgba(255, 255, 255, 0.1);
    border-radius: 10px;
    backdrop-filter: blur(18px);
    -webkit-backdrop-filter: blur(18px);
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.28);
  }

  .menu-item {
    font-family: 'Departure Mono', monospace;
    font-size: 10px;
    letter-spacing: 0.08em;
    text-align: left;
    color: rgba(255, 255, 255, 0.58);
    background: transparent;
    border: 0.5px solid transparent;
    border-radius: 6px;
    padding: 8px 10px;
    cursor: pointer;
    transition: color 0.2s, border-color 0.2s, background 0.2s;
  }

  .menu-item:hover {
    color: rgba(255, 255, 255, 0.84);
    border-color: rgba(255, 255, 255, 0.08);
    background: rgba(255, 255, 255, 0.03);
  }

  .menu-item:disabled {
    cursor: default;
    opacity: 0.5;
  }
</style>
