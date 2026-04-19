<script lang="ts">
  import type { ComposeTabInfo } from "./types";

  export let tabs: ComposeTabInfo[] = [];
  export let activeTabId = "";
  export let maxTabs = 3;
  export let selectComposeTab: (tabId: string) => void = () => {};
  export let createComposeTab: () => void = () => {};
  export let closeComposeTab: (tabId: string) => void = () => {};
</script>

<div class="compose-tabs" aria-label="compose live tabs">
  {#each tabs as tab}
    <div class="compose-tab" class:active={tab.id === activeTabId}>
      <button class="compose-tab-btn" on:click={() => selectComposeTab(tab.id)}>{tab.title}</button>
      {#if tabs.length > 1}
        <button class="compose-tab-close" aria-label="close tab" on:click={() => closeComposeTab(tab.id)}>x</button>
      {/if}
    </div>
  {/each}
  {#if tabs.length < maxTabs}
    <button class="compose-tab-add" on:click={createComposeTab}>+ tab</button>
  {/if}
</div>

<style>
  .compose-tabs {
    display: flex;
    flex-wrap: nowrap;
    gap: 5px;
    flex: 1;
    min-width: 0;
    overflow-x: auto;
    scrollbar-width: none;
  }

  .compose-tabs::-webkit-scrollbar {
    display: none;
  }

  .compose-tab {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    border-radius: 20px;
    border: 0.5px solid rgba(170, 200, 223, 0.18);
    background: rgba(18, 29, 41, 0.32);
    padding: 1px 2px;
    flex-shrink: 0;
    transition: border-color 0.2s ease, background 0.2s ease;
  }

  .compose-tab.active {
    border-color: rgba(118, 205, 184, 0.48);
    background: rgba(100, 190, 170, 0.16);
  }

  .compose-tab-btn,
  .compose-tab-close,
  .compose-tab-add {
    border: none;
    background: transparent;
    color: rgba(210, 224, 236, 0.74);
    font-family: 'Departure Mono', monospace;
    font-size: 10.5px;
    letter-spacing: 0.03em;
    text-transform: lowercase;
    cursor: pointer;
    transition: color 0.2s ease;
  }

  .compose-tab-btn {
    min-height: 28px;
    padding: 4px 11px;
    white-space: nowrap;
    max-width: 148px;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .compose-tab-close {
    min-height: 28px;
    min-width: 24px;
    padding: 4px 6px;
    color: rgba(193, 214, 230, 0.6);
  }

  .compose-tab.active .compose-tab-btn {
    color: rgba(161, 229, 212, 0.96);
  }

  .compose-tab:hover {
    border-color: rgba(159, 205, 235, 0.34);
    background: rgba(36, 58, 78, 0.46);
  }

  .compose-tab:hover .compose-tab-btn,
  .compose-tab:hover .compose-tab-close {
    color: rgba(223, 236, 245, 0.92);
  }

  .compose-tab-add {
    border-radius: 20px;
    border: 0.5px dashed rgba(170, 203, 226, 0.3);
    color: rgba(197, 214, 227, 0.64);
    min-height: 28px;
    padding: 5px 12px;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .compose-tab-add:hover {
    border-color: rgba(121, 208, 187, 0.42);
    color: rgba(170, 232, 217, 0.92);
  }

  @media (max-width: 520px) {
    .compose-tabs {
      gap: 6px;
    }

    .compose-tab-btn,
    .compose-tab-close,
    .compose-tab-add {
      font-size: 11px;
    }

    .compose-tab-btn {
      min-height: 30px;
      padding: 5px 11px;
      max-width: 136px;
    }

    .compose-tab-close {
      min-height: 30px;
      min-width: 26px;
      padding: 5px 7px;
    }

    .compose-tab-add {
      min-height: 30px;
      padding: 6px 12px;
    }
  }
</style>
