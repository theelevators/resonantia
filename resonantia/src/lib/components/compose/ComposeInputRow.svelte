<script lang="ts">
  export let draft = "";
  export let sessionId = "";
  export let loading = false;
  export let replyLoading = false;
  export let onDraftInput: () => void = () => {};
  export let sendComposeMessage: () => Promise<void> | void = () => {};
</script>

<div class="compose-entry">
  <textarea
    class="drawer-textarea compose-input"
    placeholder="message…"
    bind:value={draft}
    rows="3"
    on:input={onDraftInput}
    on:keydown={(event) => {
      if (event.key === 'Enter' && !event.shiftKey) {
        event.preventDefault();
        void sendComposeMessage();
      }
    }}
  ></textarea>
  <button
    type="button"
    class="drawer-btn submit compose-send"
    on:click={() => void sendComposeMessage()}
    disabled={loading || replyLoading || !draft.trim() || !sessionId.trim()}
  >
    {replyLoading ? 'thinking…' : 'send'}
  </button>
</div>

<style>
  .compose-entry {
    display: flex;
    gap: 8px;
    align-items: flex-end;
    flex-shrink: 0;
  }

  .drawer-textarea {
    width: 100%;
    box-sizing: border-box;
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.026), rgba(255, 255, 255, 0.015));
    border: 0.5px solid rgba(167, 196, 219, 0.2);
    border-radius: 8px;
    padding: 9px 12px;
    color: rgba(218, 230, 240, 0.96);
    font-family: 'IBM Plex Sans', sans-serif;
    font-size: 13px;
    resize: vertical;
    margin-bottom: 0;
    outline: none;
    transition: border-color 0.2s;
    line-height: 1.45;
    box-shadow: inset 0 0 0 1px rgba(108, 144, 173, 0.07);
    backdrop-filter: blur(2px);
    -webkit-backdrop-filter: blur(2px);
  }

  .drawer-textarea::placeholder {
    color: rgba(218, 231, 243, 0.44);
    font-style: italic;
  }

  .drawer-textarea:focus {
    border-color: rgba(100, 190, 170, 0.26);
    box-shadow: inset 0 0 0 1px rgba(100, 190, 170, 0.18), 0 0 0 1px rgba(100, 190, 170, 0.05);
  }

  .compose-input {
    min-height: 44px;
    max-height: 112px;
  }

  .drawer-btn {
    font-family: 'Departure Mono', monospace;
    font-size: 11.5px;
    letter-spacing: 0.05em;
    min-height: 42px;
    padding: 10px 16px;
    border-radius: 8px;
    cursor: pointer;
    transition: color 0.2s, border-color 0.2s, background 0.2s, opacity 0.2s;
    white-space: nowrap;
  }

  .drawer-btn.submit {
    background: linear-gradient(180deg, rgba(100, 190, 170, 0.18), rgba(78, 152, 136, 0.14));
    border: 0.5px solid rgba(100, 190, 170, 0.34);
    color: rgba(173, 234, 220, 0.94);
    box-shadow: 0 4px 10px rgba(16, 54, 48, 0.18);
  }

  .drawer-btn.submit:hover:not(:disabled) {
    background: rgba(100, 190, 170, 0.24);
    border-color: rgba(130, 210, 192, 0.44);
  }

  .drawer-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .compose-send {
    min-width: 96px;
  }

  @media (max-width: 520px) {
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
      min-height: 50px;
    }

    .compose-send {
      min-height: 44px;
    }
  }

  @media (hover: none) and (pointer: coarse) {
    .compose-input {
      font-size: 16px;
      line-height: 1.35;
    }

    .compose-send {
      min-height: 46px;
      font-size: 12px;
    }
  }
</style>
