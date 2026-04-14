<script lang="ts">
  import { formatTimestamp } from '@resonantia/core';

  type ComposeMode = 'live' | 'importare';

  type ComposeMessage = {
    role: 'user' | 'assistant';
    content: string;
    at: string;
  };

  type ComposeResult = {
    psi: number;
    duplicateSkipped: boolean;
    status: 'created' | 'updated' | 'duplicate' | 'skipped';
  } | null;

  export let open = false;
  export let mode: ComposeMode = 'live';
  export let sessionId = '';
  export let draft = '';
  export let messages: ComposeMessage[] = [];
  export let loading = false;
  export let replyLoading = false;
  export let encodePromptSent = false;
  export let error: string | null = null;
  export let result: ComposeResult = null;
  export let promptCopyLoading = false;
  export let promptCopied = false;
  export let promptCopyError: string | null = null;
  export let pasteNodeOpen = false;
  export let pasteNodeDraft = '';
  export let pasteNodeLoading = false;

  export let onClose: () => void = () => {};
  export let onSessionInput: () => void = () => {};
  export let sendComposeMessage: () => Promise<void> | void = () => {};
  export let copyComposeEncodePrompt: () => Promise<void> | void = () => {};
  export let toggleComposePasteNode: () => void = () => {};
  export let clearComposeConversation: () => void = () => {};
  export let switchComposeToLive: () => void = () => {};
  export let saveComposePastedNode: () => Promise<void> | void = () => {};
  export let submitCompose: () => Promise<void> | void = () => {};

  function composeOutcomeLabel(status: 'created' | 'updated' | 'duplicate' | 'skipped', duplicateSkipped: boolean) {
    if (duplicateSkipped || status === 'duplicate' || status === 'skipped') {
      return 'already present · duplicate skipped';
    }
    if (status === 'updated') {
      return 'updated';
    }
    return 'stored';
  }
</script>

{#if open}
  <div class="drawer drawer-compose" class:importare={mode === 'importare'} role="dialog" aria-label={mode === 'importare' ? 'Import node' : 'Compose context'}>
    <div class="drawer-header">
      <span class="drawer-title">{mode === 'importare' ? 'importare' : 'compose'}</span>
      <button class="close-btn" on:click={onClose}>✕</button>
    </div>
    <input
      class="drawer-input"
      type="text"
      placeholder="session id (required)"
      bind:value={sessionId}
      on:input={onSessionInput}
    />
    {#if mode === 'live'}
      <div class="compose-thread" aria-live="polite">
        {#if messages.length === 0}
          <p class="compose-empty">chat first, then encode the thread into one protocol node.</p>
        {:else}
          {#each messages as message}
            <article class={`compose-bubble ${message.role === 'assistant' ? 'assistant' : 'user'}`}>
              <header class="compose-bubble-meta">
                <span>{message.role === 'assistant' ? 'resonare' : 'you'}</span>
                <small>{formatTimestamp(message.at)}</small>
              </header>
              <p>{message.content}</p>
            </article>
          {/each}
        {/if}

        {#if replyLoading}
          <article class="compose-bubble assistant compose-pending">
            <header class="compose-bubble-meta">
              <span>resonare</span>
              <small>thinking…</small>
            </header>
          </article>
        {/if}
      </div>

      <div class="compose-entry">
        <textarea
          class="drawer-textarea compose-input"
          placeholder="message…"
          bind:value={draft}
          rows="3"
          on:keydown={(event) => {
            if (event.key === 'Enter' && !event.shiftKey) {
              event.preventDefault();
              void sendComposeMessage();
            }
          }}
        ></textarea>
        <button
          class="drawer-btn submit compose-send"
          on:click={sendComposeMessage}
          disabled={loading || replyLoading || !draft.trim() || !sessionId.trim()}
        >
          {replyLoading ? 'thinking…' : 'send'}
        </button>
      </div>
    {:else}
      <p class="compose-importare-note">paste one complete node and store it directly.</p>
    {/if}

    <div class="compose-utility-actions">
      <button class="compose-link-btn" on:click={copyComposeEncodePrompt} disabled={promptCopyLoading || loading || replyLoading}>
        {promptCopyLoading ? 'copying distill prompt…' : promptCopied ? 'distill prompt copied' : 'copy distill prompt'}
      </button>
      {#if mode === 'live'}
        <span class="compose-utility-divider">•</span>
        <button class="compose-link-btn" on:click={toggleComposePasteNode} disabled={pasteNodeLoading || loading || replyLoading}>
          {pasteNodeOpen ? 'hide paste save' : 'paste node to save'}
        </button>
        {#if messages.length > 0}
          <span class="compose-utility-divider">•</span>
          <button class="compose-link-btn" on:click={clearComposeConversation} disabled={loading || replyLoading}>clear thread</button>
        {/if}
      {:else}
        <span class="compose-utility-divider">•</span>
        <button class="compose-link-btn" data-tour-target="compose-switch-live" on:click={switchComposeToLive} disabled={pasteNodeLoading}>switch to create live</button>
      {/if}
    </div>
    {#if promptCopyError}
      <p class="drawer-error">copy failed: {promptCopyError}</p>
    {/if}
    {#if mode === 'importare' || pasteNodeOpen}
      <div class="compose-paste-panel">
        <p class="compose-paste-intro">paste a complete STTP node and save it directly.</p>
        <textarea
          class="drawer-textarea compose-paste-input"
          placeholder="paste one full STTP node"
          bind:value={pasteNodeDraft}
          rows="9"
        ></textarea>
        <div class="compose-paste-actions">
          {#if mode === 'live'}
            <button class="drawer-btn cancel" on:click={toggleComposePasteNode} disabled={pasteNodeLoading}>cancel paste</button>
          {/if}
          <button class="drawer-btn submit" on:click={saveComposePastedNode} disabled={pasteNodeLoading || !pasteNodeDraft.trim() || !sessionId.trim()}>
            {pasteNodeLoading ? 'saving…' : 'save pasted node'}
          </button>
        </div>
      </div>
    {/if}
    {#if mode === 'live' && loading && encodePromptSent}
      <p class="drawer-success compose-encode-note">encoding prompt sent</p>
    {/if}
    {#if error}<p class="drawer-error">{error}</p>{/if}
    {#if result}
      <p class="drawer-success">
        {composeOutcomeLabel(result.status, result.duplicateSkipped)} · Ψ {result.psi.toFixed(4)}
      </p>
    {/if}
    <div class="drawer-actions compose-actions">
      <button class="drawer-btn cancel" on:click={onClose}>{mode === 'importare' ? 'close' : 'cancel'}</button>
      {#if mode === 'live'}
        <button class="drawer-btn submit" on:click={submitCompose} disabled={loading || replyLoading || messages.length === 0 || !sessionId.trim()}>
          {loading ? 'encoding…' : 'encode + store'}
        </button>
      {/if}
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

  .drawer-compose {
    top: max(64px, calc(var(--safe-top) + 46px));
    bottom: auto;
    max-height: min(680px, calc(100dvh - 150px));
  }

  .drawer-compose.importare {
    max-height: min(620px, calc(100dvh - 160px));
  }

  .compose-thread {
    min-height: 144px;
    max-height: 290px;
    overflow-y: auto;
    border: 0.5px solid rgba(255, 255, 255, 0.06);
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.018);
    padding: 9px;
    margin-bottom: 10px;
    display: flex;
    flex-direction: column;
    gap: 7px;
  }

  .compose-empty {
    margin: auto 0;
    text-align: center;
    font-size: 10px;
    line-height: 1.5;
    color: rgba(255, 255, 255, 0.34);
    letter-spacing: 0.03em;
  }

  .compose-bubble {
    border: 0.5px solid rgba(255, 255, 255, 0.065);
    border-radius: 9px;
    padding: 6px 8px;
    background: rgba(255, 255, 255, 0.03);
  }

  .compose-bubble.user {
    border-color: rgba(255, 255, 255, 0.18);
    background: rgba(255, 255, 255, 0.06);
  }

  .compose-bubble.assistant {
    border-color: rgba(214, 184, 109, 0.26);
    background: rgba(214, 184, 109, 0.08);
  }

  .compose-bubble-meta {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 8px;
    margin-bottom: 5px;
    font-size: 9px;
    letter-spacing: 0.08em;
    text-transform: lowercase;
    color: rgba(255, 255, 255, 0.5);
  }

  .compose-bubble-meta small {
    font-size: 8px;
    letter-spacing: 0.05em;
    color: rgba(255, 255, 255, 0.35);
  }

  .compose-bubble p {
    margin: 0;
    font-size: 10px;
    line-height: 1.45;
    color: rgba(255, 255, 255, 0.76);
    white-space: pre-wrap;
    word-break: break-word;
  }

  .compose-pending {
    animation: composePulse 1.4s ease-in-out infinite;
  }

  .compose-entry {
    display: flex;
    gap: 6px;
    align-items: flex-end;
  }

  .compose-input {
    margin-bottom: 0;
    min-height: 84px;
  }

  .compose-send {
    min-width: 88px;
    margin-bottom: 1px;
  }

  .compose-importare-note {
    margin: 0 0 8px;
    font-size: 10px;
    line-height: 1.45;
    letter-spacing: 0.04em;
    color: rgba(255, 255, 255, 0.44);
    text-transform: lowercase;
  }

  .compose-utility-actions {
    display: flex;
    align-items: center;
    gap: 7px;
    margin-top: 7px;
    margin-bottom: 2px;
  }

  .compose-utility-divider {
    color: rgba(255, 255, 255, 0.18);
    font-size: 9px;
    line-height: 1;
    user-select: none;
  }

  .compose-link-btn {
    border: none;
    background: transparent;
    padding: 0;
    margin: 0;
    font-family: 'Departure Mono', monospace;
    font-size: 9px;
    letter-spacing: 0.04em;
    text-transform: lowercase;
    color: rgba(255, 255, 255, 0.46);
    cursor: pointer;
    transition: color 0.2s;
  }

  .compose-link-btn:hover:not(:disabled) {
    color: rgba(255, 255, 255, 0.76);
  }

  .compose-link-btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .compose-paste-panel {
    margin-top: 7px;
    padding: 10px;
    border-radius: 10px;
    border: 0.5px dashed rgba(255, 255, 255, 0.11);
    background: rgba(255, 255, 255, 0.012);
  }

  .compose-paste-intro {
    margin: 0 0 8px;
    font-size: 9px;
    line-height: 1.45;
    letter-spacing: 0.04em;
    color: rgba(255, 255, 255, 0.48);
    text-transform: lowercase;
  }

  .compose-paste-input {
    min-height: 168px;
    margin-bottom: 0;
  }

  .drawer-compose.importare .compose-paste-input {
    min-height: 224px;
  }

  .compose-paste-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 8px;
  }

  .compose-actions {
    justify-content: flex-end;
    align-items: center;
  }

  .compose-encode-note {
    margin-top: 8px;
    opacity: 0.85;
    letter-spacing: 0.04em;
    text-transform: lowercase;
  }

  @keyframes composePulse {
    0%,
    100% {
      opacity: 0.64;
    }
    50% {
      opacity: 1;
    }
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

  .drawer-textarea {
    width: 100%;
    box-sizing: border-box;
    background: rgba(255, 255, 255, 0.04);
    border: 0.5px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    padding: 8px 10px;
    color: rgba(255, 255, 255, 0.7);
    font-family: 'Departure Mono', monospace;
    font-size: 11px;
    resize: vertical;
    margin-bottom: 10px;
    outline: none;
    transition: border-color 0.2s;
  }

  .drawer-textarea:focus {
    border-color: rgba(255, 255, 255, 0.25);
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
      top: calc(var(--safe-top) + 56px);
      bottom: max(74px, calc(var(--safe-bottom) + 58px));
      width: calc(100vw - 20px);
      max-height: calc(100dvh - 130px);
      padding: 16px;
    }

    .drawer-compose {
      top: calc(var(--safe-top) + 56px);
      bottom: auto;
      max-height: min(74dvh, 560px);
      padding: 14px;
    }

    .compose-thread {
      max-height: 236px;
      min-height: 132px;
    }

    .compose-entry {
      flex-direction: column;
      align-items: stretch;
      gap: 6px;
    }

    .compose-send {
      width: 100%;
      min-width: 0;
    }

    .compose-input {
      min-height: 72px;
    }

    .compose-utility-actions,
    .compose-paste-actions {
      flex-direction: column;
      align-items: stretch;
    }

    .compose-link-btn,
    .compose-paste-actions .drawer-btn {
      width: 100%;
      text-align: left;
      padding: 1px 0;
    }

    .compose-utility-divider {
      display: none;
    }
  }
</style>
