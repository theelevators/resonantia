<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let faded = false;
  export let hidden = false;

  const dispatch = createEventDispatcher<{
    open: void;
  }>();
</script>

<div class="compose-launch" class:faded={faded} class:hidden={hidden}>
  <button class="compose-btn" data-tour-target="compose-live" on:click={() => dispatch('open')}>+ compose</button>
</div>

<style>
  .compose-launch {
    position: absolute;
    bottom: max(24px, calc(env(safe-area-inset-bottom, 0px) + 12px));
    right: max(24px, calc(env(safe-area-inset-right, 0px) + 12px));
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 6px;
    z-index: 10;
    transition: all 0.2s;
  }

  .compose-launch.faded {
    opacity: 0;
    transform: translateY(8px);
    pointer-events: none;
  }

  .compose-launch.hidden {
    opacity: 0;
    transform: translateY(8px);
    pointer-events: none;
  }

  .compose-btn {
    font-family: 'Departure Mono', monospace;
    font-size: 12px;
    letter-spacing: 0.1em;
    color: rgba(225, 236, 245, 0.9);
    background: rgba(12, 18, 27, 0.9);
    border: 0.5px solid rgba(176, 201, 220, 0.3);
    border-radius: 8px;
    min-height: 42px;
    padding: 11px 20px;
    cursor: pointer;
    transition: all 0.2s;
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    box-shadow: 0 8px 18px rgba(4, 9, 16, 0.28);
  }

  .compose-btn:hover {
    color: rgba(245, 250, 255, 0.98);
    border-color: rgba(200, 223, 240, 0.52);
    background: rgba(28, 41, 58, 0.96);
  }

  .compose-btn:focus-visible {
    outline: 1px solid rgba(192, 223, 244, 0.82);
    outline-offset: 1px;
  }

  @media (max-width: 640px) {
    .compose-launch {
      bottom: max(18px, calc(env(safe-area-inset-bottom, 0px) + 10px));
      right: max(14px, calc(env(safe-area-inset-right, 0px) + 10px));
    }

    .compose-btn {
      min-height: 46px;
      font-size: 12.5px;
      padding: 12px 18px;
    }
  }
</style>
