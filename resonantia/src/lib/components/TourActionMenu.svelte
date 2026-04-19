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
    font-size: 11px;
    letter-spacing: 0.08em;
    color: rgba(221, 232, 240, 0.76);
    background: rgba(18, 24, 34, 0.5);
    border: 0.5px solid rgba(175, 199, 214, 0.28);
    border-radius: 4px;
    min-height: 34px;
    padding: 6px 12px;
    cursor: pointer;
    transition: color 0.2s, border-color 0.2s, background 0.2s;
  }

  .nav-btn:hover {
    color: rgba(240, 246, 250, 0.98);
    border-color: rgba(196, 220, 236, 0.48);
    background: rgba(43, 58, 76, 0.58);
  }

  .menu-wrap {
    position: relative;
  }

  .menu-btn {
    min-width: 42px;
    padding: 6px 0;
    text-align: center;
    font-size: 16px;
    line-height: 1;
  }

  .menu-btn.open {
    color: rgba(242, 248, 251, 0.98);
    border-color: rgba(201, 223, 238, 0.56);
    background: rgba(56, 73, 92, 0.62);
  }

  .menu-popover {
    position: absolute;
    top: calc(100% + 8px);
    right: 0;
    width: 178px;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    background: rgba(10, 13, 19, 0.98);
    border: 0.5px solid rgba(183, 205, 220, 0.24);
    border-radius: 10px;
    backdrop-filter: blur(18px);
    -webkit-backdrop-filter: blur(18px);
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.28);
  }

  .menu-item {
    font-family: 'Departure Mono', monospace;
    font-size: 11px;
    letter-spacing: 0.08em;
    text-align: left;
    color: rgba(214, 228, 237, 0.9);
    background: transparent;
    border: 0.5px solid transparent;
    border-radius: 6px;
    min-height: 36px;
    padding: 9px 11px;
    cursor: pointer;
    transition: color 0.2s, border-color 0.2s, background 0.2s;
  }

  .menu-item:hover {
    color: rgba(241, 247, 251, 0.98);
    border-color: rgba(179, 205, 223, 0.28);
    background: rgba(55, 75, 95, 0.38);
  }

  .nav-btn:focus-visible,
  .menu-item:focus-visible {
    outline: 1px solid rgba(188, 221, 244, 0.78);
    outline-offset: 1px;
  }

  .menu-item:disabled {
    cursor: default;
    opacity: 0.5;
  }

  @media (max-width: 640px) {
    .nav-btn {
      min-height: 38px;
      font-size: 11.5px;
    }

    .menu-btn {
      min-width: 44px;
      font-size: 17px;
    }

    .menu-popover {
      width: min(204px, calc(100vw - 24px));
      top: calc(100% + 10px);
    }

    .menu-item {
      min-height: 38px;
      font-size: 11.5px;
      padding: 10px 11px;
    }
  }
</style>
