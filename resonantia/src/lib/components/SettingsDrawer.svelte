<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let open = false;
  export let loading = false;
  export let saving = false;
  export let error: string | null = null;
  export let saved = false;
  export let localModelOriginWarning: string | null = null;
  export let ollamaBaseUrl = '';
  export let ollamaModel = '';
  export let gatewayBaseUrl = '';
  export let syncAdvancedOpen = false;

  const dispatch = createEventDispatcher<{
    close: void;
    save: void;
    demo: void;
  }>();
</script>

{#if open}
  <div class="drawer" role="dialog" aria-label="Settings">
    <div class="drawer-header">
      <span class="drawer-title">settings</span>
      <button class="close-btn" on:click={() => dispatch('close')}>✕</button>
    </div>
    <p class="settings-intro">Resonantia runs local-first. Model settings live here, and cloud sync can be linked once in advanced settings.</p>
    <button
      class="settings-advanced-toggle"
      on:click={() => dispatch('demo')}
      disabled={loading || saving}
    >
      run cinematic demo
    </button>

    <label class="settings-field">
      <span class="settings-label">ollama base url</span>
      <span class="settings-note">Local model endpoint for transmutation and summaries</span>
      <input class="drawer-input" type="text" placeholder="http://127.0.0.1:11434" bind:value={ollamaBaseUrl} disabled={loading || saving} />
    </label>
    {#if localModelOriginWarning}
      <p class="drawer-error settings-inline-warning">{localModelOriginWarning}</p>
    {/if}

    <label class="settings-field">
      <span class="settings-label">ollama model</span>
      <span class="settings-note">Model name Resonantia should call by default</span>
      <input class="drawer-input" type="text" placeholder="llama3.2" bind:value={ollamaModel} disabled={loading || saving} />
    </label>

    <button
      class="settings-advanced-toggle"
      on:click={() => (syncAdvancedOpen = !syncAdvancedOpen)}
      disabled={loading || saving}
    >
      {syncAdvancedOpen ? 'hide advanced sync' : 'advanced sync'}
    </button>

    {#if syncAdvancedOpen}
      <div class="settings-advanced-panel">
        <label class="settings-field">
          <span class="settings-label">cloud sync path</span>
          <span class="settings-note">Set this once, then just use Sync from the menu.</span>
          <input
            class="drawer-input"
            type="text"
            placeholder="https://your-sync-endpoint"
            bind:value={gatewayBaseUrl}
            disabled={loading || saving}
          />
        </label>
      </div>
    {/if}

    {#if loading}<p class="drawer-success">loading config…</p>{/if}
    {#if error}<p class="drawer-error">{error}</p>{/if}
    {#if saved}<p class="drawer-success">settings saved</p>{/if}
    <div class="drawer-actions">
      <button class="drawer-btn cancel" on:click={() => dispatch('close')}>cancel</button>
      <button class="drawer-btn submit" on:click={() => dispatch('save')} disabled={loading || saving}>
        {saving ? 'saving…' : 'save'}
      </button>
    </div>
  </div>
{/if}

<style>
  .drawer {
    position: absolute;
    top: 64px;
    bottom: 84px;
    left: 50%;
    transform: translateX(-50%);
    width: min(456px, calc(100vw - 32px));
    max-height: calc(100dvh - 148px);
    overflow-y: auto;
    background: rgba(10, 11, 14, 0.97);
    border: 0.5px solid rgba(255, 255, 255, 0.1);
    border-radius: 14px;
    padding: 20px;
    z-index: 20;
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    font-family: 'Departure Mono', 'Courier New', monospace;
    overscroll-behavior: contain;
    scrollbar-width: thin;
  }

  .drawer-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 14px;
  }

  .drawer-title {
    font-family: 'Fraunces', Georgia, serif;
    font-weight: 300;
    font-style: italic;
    font-size: 15px;
    color: rgba(255, 255, 255, 0.55);
  }

  .close-btn {
    background: transparent;
    border: none;
    color: rgba(255, 255, 255, 0.3);
    font-size: 14px;
    cursor: pointer;
    padding: 0;
    transition: color 0.2s;
  }

  .close-btn:hover {
    color: rgba(255, 255, 255, 0.7);
  }

  .drawer-input {
    width: 100%;
    box-sizing: border-box;
    background: rgba(255, 255, 255, 0.04);
    border: 0.5px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    padding: 8px 10px;
    color: rgba(255, 255, 255, 0.7);
    font-family: 'Departure Mono', monospace;
    font-size: 11px;
    margin-bottom: 10px;
    outline: none;
    transition: border-color 0.2s;
  }

  .drawer-input:focus {
    border-color: rgba(255, 255, 255, 0.25);
  }

  .settings-intro {
    margin: -2px 0 10px;
    font-size: 10px;
    line-height: 1.5;
    color: rgba(255, 255, 255, 0.45);
  }

  .settings-field {
    display: grid;
    gap: 4px;
    margin-bottom: 10px;
    align-items: start;
  }

  .settings-label {
    font-size: 9px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: rgba(255, 255, 255, 0.56);
  }

  .settings-note {
    font-size: 9px;
    line-height: 1.45;
    color: rgba(255, 255, 255, 0.36);
  }

  .settings-field .drawer-input {
    margin-bottom: 0;
  }

  .settings-advanced-toggle {
    margin: 0 0 10px;
    width: 100%;
    background: rgba(255, 255, 255, 0.02);
    border: 0.5px dashed rgba(255, 255, 255, 0.12);
    border-radius: 7px;
    color: rgba(255, 255, 255, 0.5);
    padding: 7px 9px;
    font-family: 'Departure Mono', monospace;
    font-size: 9px;
    letter-spacing: 0.08em;
    text-transform: lowercase;
    text-align: left;
    cursor: pointer;
    transition: border-color 0.2s, color 0.2s, background 0.2s;
  }

  .settings-advanced-toggle:hover:not(:disabled) {
    border-color: rgba(255, 255, 255, 0.26);
    color: rgba(255, 255, 255, 0.78);
    background: rgba(255, 255, 255, 0.04);
  }

  .settings-advanced-toggle:disabled {
    opacity: 0.55;
    cursor: default;
  }

  .settings-advanced-panel {
    margin-bottom: 10px;
    padding: 11px;
    border-radius: 9px;
    border: 0.5px solid rgba(255, 255, 255, 0.08);
    background: rgba(255, 255, 255, 0.02);
  }

  .settings-inline-warning {
    margin: -2px 0 9px;
    line-height: 1.45;
  }

  .drawer-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 12px;
  }

  .drawer-btn {
    font-family: 'Departure Mono', monospace;
    font-size: 10px;
    letter-spacing: 0.08em;
    padding: 6px 14px;
    border-radius: 5px;
    cursor: pointer;
    transition: all 0.2s;
  }

  .drawer-btn.cancel {
    background: transparent;
    border: 0.5px solid rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.3);
  }

  .drawer-btn.submit {
    background: rgba(255, 255, 255, 0.06);
    border: 0.5px solid rgba(255, 255, 255, 0.2);
    color: rgba(255, 255, 255, 0.8);
  }

  .drawer-btn.submit:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.1);
    border-color: rgba(255, 255, 255, 0.35);
  }

  .drawer-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .drawer-error {
    font-size: 10px;
    color: rgba(233, 148, 58, 0.8);
    margin: 6px 0 0;
  }

  .drawer-success {
    font-size: 10px;
    color: rgba(122, 170, 122, 0.9);
    margin: 6px 0 0;
  }

  @media (max-width: 520px) {
    .drawer {
      top: calc(env(safe-area-inset-top, 0px) + 56px);
      bottom: max(74px, calc(env(safe-area-inset-bottom, 0px) + 58px));
      width: calc(100vw - 20px);
      max-height: calc(100dvh - 130px);
      padding: 16px;
    }
  }
</style>
