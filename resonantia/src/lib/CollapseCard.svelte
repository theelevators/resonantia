<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { AVEC_HEX, avecColor, formatTimestamp } from './avec';
  import type { AiSummary, CollapseCardData } from './types';

  export let data: CollapseCardData | null = null;
  export let visible = false;
  export let summary: AiSummary | null = null;
  export let transmuting = false;
  export let transmuteError: string | null = null;

  const dispatch = createEventDispatcher<{
    close: void;
    navigate: { sessionId: string };
    transmute: void;
  }>();

  $: avec = data?.nodeDto?.userAvec ?? null;

  $: sessionLabel = data?.node.sessionId.replace(/_/g, ' ') ?? '—';
  $: timestamp    = data?.node.timestamp ? formatTimestamp(data.node.timestamp) : '—';
  $: tier         = data?.node.tier ?? '—';
  let summaryOpen = false;
  $: if (summary) summaryOpen = true;
  $: shellStyle = [
    'position:fixed',
    'bottom:18px',
    'left:50%',
    `transform:translateX(-50%) translateY(${visible ? '0' : 'calc(100% + 32px)'})`,
    `opacity:${visible ? '1' : '0'}`,
    'width:min(312px, calc(100vw - 28px))',
    'max-width:calc(100vw - 32px)',
    'background:rgba(12, 14, 20, 0.96)',
    'border:0.5px solid rgba(255, 255, 255, 0.1)',
    'border-radius:13px',
    'padding:16px',
    'z-index:30',
    'box-shadow:0 18px 44px rgba(0, 0, 0, 0.34)',
    'backdrop-filter:blur(20px)',
    '-webkit-backdrop-filter:blur(20px)',
    `pointer-events:${visible ? 'all' : 'none'}`,
    'font-family:"Departure Mono", "Courier New", monospace',
    'transition:transform 0.5s cubic-bezier(0.16, 1, 0.3, 1), opacity 0.4s ease'
  ].join(';');

  function bar(val: number | null) {
    return `${((val ?? 0) * 100).toFixed(0)}%`;
  }

  function handleTransmute() {
    if (summary) {
      summaryOpen = !summaryOpen;
      return;
    }
    dispatch('transmute');
  }
</script>

<div
  class="whisper-card"
  class:visible
  style={shellStyle}
  role="complementary"
  aria-label="Collapse detail"
>
  <div class="whisper-header">
    <div class="whisper-left">
      <span class="whisper-session-label">{sessionLabel}</span>
      <span class="whisper-tier-pill">{tier}</span>
    </div>
    <button class="whisper-close-btn" on:click={() => dispatch('close')} aria-label="Close">✕</button>
  </div>

  <div class="whisper-timestamp">{timestamp}</div>

  <div class="whisper-actions whisper-actions-top">
    <button
      class="whisper-transmute-btn"
      on:click={handleTransmute}
      disabled={!data?.nodeDto || transmuting}
    >
      {transmuting ? 'transmuting…' : 'transmute'}
    </button>

    {#if summary && summaryOpen}
      <div class="alchemy-card" role="region" aria-label="Transmutation">
        <div class="alchemy-header">
          <span class="alchemy-title">transmutation</span>
          <button class="alchemy-close-btn" on:click={() => (summaryOpen = false)} aria-label="Close transmutation">✕</button>
        </div>

        <div class="alchemy-body">
          {#if summary.topic}
            <div class="alchemy-block">
              <span class="alchemy-label">topic</span>
              <p class="alchemy-text">{summary.topic}</p>
            </div>
          {/if}
          {#if summary.whatHappened}
            <div class="alchemy-block">
              <span class="alchemy-label">what happened</span>
              <p class="alchemy-text">{summary.whatHappened}</p>
            </div>
          {/if}
          {#if summary.whereWeLeftOff}
            <div class="alchemy-block">
              <span class="alchemy-label">where we left off</span>
              <p class="alchemy-text">{summary.whereWeLeftOff}</p>
            </div>
          {/if}
          {#if summary.vibe}
            <div class="alchemy-block">
              <span class="alchemy-label">vibe</span>
              <p class="alchemy-text">{summary.vibe}</p>
            </div>
          {/if}
          {#if summary.pickBackUpWith}
            <div class="alchemy-block">
              <span class="alchemy-label">pick back up with</span>
              <p class="alchemy-text">{summary.pickBackUpWith}</p>
            </div>
          {/if}
        </div>
      </div>
    {/if}
  </div>

  {#if data?.relatedSessions?.length}
    <div class="whisper-threads whisper-threads-top">
      {#each data.relatedSessions.slice(0, 4) as s}
        <button
          class="whisper-thread-tag"
          on:click={() => dispatch('navigate', { sessionId: s.id })}
        >
          ⟶ {s.label.split('_').slice(0, 2).join(' ')}
        </button>
      {/each}
    </div>
  {/if}

  {#if avec}
    <div class="whisper-avec-grid">
      {#each [
        { key: 'stability' as const, label: 'grounding', val: avec.stability },
        { key: 'friction'  as const, label: 'wear', val: avec.friction  },
        { key: 'logic'     as const, label: 'clarity', val: avec.logic      },
        { key: 'autonomy'  as const, label: 'self-trust', val: avec.autonomy   },
      ] as dim}
        <div class="whisper-dim">
          <span class="whisper-dim-label">{dim.label}</span>
          <div class="whisper-bar-track">
            <div
              class="whisper-bar-fill"
              style="width:{bar(dim.val)};background:{AVEC_HEX[dim.key]}"
            ></div>
          </div>
          <span class="whisper-dim-val">{dim.val.toFixed(2)}</span>
        </div>
      {/each}
    </div>
  {/if}

  {#if transmuteError}
    <p class="whisper-transmute-error">{transmuteError}</p>
  {/if}
</div>

<style>
  .whisper-card {
    will-change: transform, opacity;
    overflow: visible;
  }

  .whisper-card.visible {
    transform: translateX(-50%) translateY(0);
    opacity: 1;
    pointer-events: all;
  }

  .whisper-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    margin-bottom: 4px;
  }

  .whisper-left {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .whisper-session-label {
    font-family: 'Fraunces', Georgia, serif;
    font-weight: 300;
    font-style: italic;
    font-size: 13px;
    color: rgba(255, 255, 255, 0.75);
    line-height: 1.3;
    max-width: 220px;
  }

  .whisper-tier-pill {
    display: inline-block;
    font-size: 9px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: rgba(255, 255, 255, 0.25);
    border: 0.5px solid rgba(255, 255, 255, 0.1);
    border-radius: 99px;
    padding: 2px 7px;
    align-self: flex-start;
  }

  .whisper-close-btn {
    background: transparent;
    border: none;
    color: rgba(255, 255, 255, 0.3);
    font-size: 14px;
    cursor: pointer;
    padding: 0;
    line-height: 1;
    transition: color 0.2s;
  }
  .whisper-close-btn:hover { color: rgba(255, 255, 255, 0.7); }

  .whisper-timestamp {
    font-size: 10px;
    color: rgba(255, 255, 255, 0.25);
    letter-spacing: 0.06em;
    margin-bottom: 12px;
  }

  .whisper-avec-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    margin-top: 12px;
  }

  .whisper-dim {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .whisper-dim-label {
    font-size: 9px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: rgba(255, 255, 255, 0.3);
  }

  .whisper-bar-track {
    height: 2px;
    background: rgba(255, 255, 255, 0.06);
    border-radius: 2px;
    overflow: hidden;
  }

  .whisper-bar-fill {
    height: 100%;
    border-radius: 2px;
    transition: width 0.7s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .whisper-dim-val {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.5);
  }

  .whisper-actions {
    margin-top: 12px;
    position: relative;
    display: inline-flex;
    flex-direction: column;
    align-items: flex-start;
  }

  .whisper-actions-top {
    margin-top: 0;
  }

  .whisper-transmute-btn {
    font-family: 'Departure Mono', monospace;
    font-size: 10px;
    letter-spacing: 0.09em;
    text-transform: uppercase;
    padding: 8px 12px;
    border-radius: 999px;
    border: 0.5px solid rgba(199, 182, 132, 0.28);
    background: rgba(196, 166, 104, 0.08);
    color: rgba(229, 214, 182, 0.78);
    cursor: pointer;
    transition: color 0.2s, border-color 0.2s, background 0.2s, opacity 0.2s;
  }

  .whisper-transmute-btn:hover:not(:disabled) {
    color: rgba(247, 235, 210, 0.9);
    border-color: rgba(215, 191, 136, 0.42);
    background: rgba(196, 166, 104, 0.13);
  }

  .whisper-transmute-btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .whisper-transmute-error {
    margin-top: 10px;
    font-size: 10px;
    line-height: 1.5;
    color: rgba(233, 148, 58, 0.82);
  }

  .alchemy-card {
    position: absolute;
    left: 0;
    bottom: calc(100% + 14px);
    width: min(360px, calc(100vw - 56px));
    max-width: calc(100vw - 56px);
    padding: 14px 14px 12px;
    border-radius: 14px;
    border: 0.5px solid rgba(198, 168, 108, 0.22);
    background:
      linear-gradient(180deg, rgba(38, 29, 18, 0.96), rgba(18, 15, 12, 0.98)),
      radial-gradient(circle at top left, rgba(196, 166, 104, 0.14), transparent 55%);
    box-shadow: 0 14px 32px rgba(0, 0, 0, 0.34);
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
  }

  .alchemy-card::after {
    content: '';
    position: absolute;
    left: 34px;
    top: 100%;
    width: 14px;
    height: 14px;
    transform: translateY(-7px) rotate(45deg);
    background: rgba(22, 18, 14, 0.96);
    border-right: 0.5px solid rgba(198, 168, 108, 0.22);
    border-bottom: 0.5px solid rgba(198, 168, 108, 0.22);
  }

  .alchemy-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin-bottom: 10px;
  }

  .alchemy-title {
    font-size: 10px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: rgba(229, 214, 182, 0.72);
  }

  .alchemy-close-btn {
    background: transparent;
    border: none;
    color: rgba(229, 214, 182, 0.42);
    font-size: 12px;
    cursor: pointer;
    padding: 0;
  }

  .alchemy-body {
    display: flex;
    flex-direction: column;
    gap: 10px;
    max-height: 260px;
    overflow: auto;
    padding-right: 4px;
  }

  .alchemy-block {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .alchemy-label {
    font-size: 9px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: rgba(207, 194, 160, 0.42);
  }

  .alchemy-text {
    font-size: 11px;
    line-height: 1.65;
    color: rgba(240, 232, 214, 0.8);
  }

  .whisper-threads {
    margin-top: 12px;
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .whisper-threads-top {
    margin-top: 10px;
    margin-bottom: 2px;
  }

  .whisper-thread-tag {
    font-family: 'Departure Mono', monospace;
    font-size: 9px;
    letter-spacing: 0.06em;
    padding: 3px 9px;
    border-radius: 99px;
    border: 0.5px solid rgba(255, 255, 255, 0.1);
    background: transparent;
    color: rgba(255, 255, 255, 0.3);
    cursor: pointer;
    transition: all 0.2s;
  }
  .whisper-thread-tag:hover {
    border-color: rgba(255, 255, 255, 0.25);
    color: rgba(255, 255, 255, 0.7);
  }

  @media (max-width: 640px) {
    .whisper-card {
      bottom: 14px;
      padding: 14px;
      border-radius: 12px;
    }

    .alchemy-card {
      left: 0;
      width: min(300px, calc(100vw - 48px));
      max-width: calc(100vw - 48px);
      bottom: calc(100% + 10px);
    }

    .alchemy-text {
      font-size: 10px;
    }
  }
</style>
