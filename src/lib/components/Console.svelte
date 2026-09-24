<script lang="ts">
  import { tauriInvoke } from '$lib/stores/redis';
  import Icons from './Icons.svelte';

  interface HistoryEntry {
    command: string;
    result: string;
    isError: boolean;
    timestamp: number;
  }

  let inputValue = '';
  let history: HistoryEntry[] = [];
  let commandHistory: string[] = [];
  let historyIndex = -1;
  let executing = false;
  let outputEl: HTMLDivElement;
  let consoleFilter = '';

  $: filteredHistory = consoleFilter.trim()
    ? history.filter((entry) =>
        entry.command.toLowerCase().includes(consoleFilter.trim().toLowerCase()) ||
        entry.result.toLowerCase().includes(consoleFilter.trim().toLowerCase())
      )
    : history;

  async function executeCommand() {
    const cmd = inputValue.trim();
    if (!cmd || executing) return;

    commandHistory = [cmd, ...commandHistory.slice(0, 99)];
    historyIndex = -1;
    inputValue = '';
    executing = true;

    try {
      const result = await tauriInvoke<string>('execute_command', { command: cmd });
      history = [...history, {
        command: cmd,
        result,
        isError: false,
        timestamp: Date.now(),
      }];
    } catch (e: any) {
      history = [...history, {
        command: cmd,
        result: e.toString(),
        isError: true,
        timestamp: Date.now(),
      }];
    }

    executing = false;
    // Scroll to bottom
    setTimeout(() => {
      if (outputEl) outputEl.scrollTop = outputEl.scrollHeight;
    }, 10);
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      executeCommand();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (historyIndex < commandHistory.length - 1) {
        historyIndex++;
        inputValue = commandHistory[historyIndex];
      }
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (historyIndex > 0) {
        historyIndex--;
        inputValue = commandHistory[historyIndex];
      } else {
        historyIndex = -1;
        inputValue = '';
      }
    }
  }

  function clearHistory() {
    history = [];
  }

  // Common command suggestions
  const suggestions = [
    { cmd: 'PING', desc: 'Test connection' },
    { cmd: 'INFO server', desc: 'Server info' },
    { cmd: 'DBSIZE', desc: 'Total keys' },
    { cmd: 'KEYS *pattern*', desc: 'Find keys' },
    { cmd: 'GET key', desc: 'Get string' },
    { cmd: 'HGETALL key', desc: 'Get hash' },
    { cmd: 'LRANGE key 0 -1', desc: 'Get list' },
    { cmd: 'SMEMBERS key', desc: 'Get set' },
    { cmd: 'TTL key', desc: 'Get TTL' },
    { cmd: 'TYPE key', desc: 'Get type' },
  ];
</script>

<div class="console">
  <div class="console-header">
    <span class="console-title" style="display: inline-flex; align-items: center; gap: 6px;">
      <Icons name="terminal" size={14} />
      <span>Redis CLI</span>
    </span>
    <div class="console-actions">
      {#if history.length > 0}
        <div class="console-filter-wrap">
          <input
            class="input input-sm console-filter-input"
            bind:value={consoleFilter}
            placeholder="Filter output..."
          />
          {#if consoleFilter}
            <button class="clear-mini-btn" on:click={() => consoleFilter = ''}>✕</button>
          {/if}
        </div>
      {/if}
      <button class="btn btn-sm" on:click={clearHistory} disabled={history.length === 0}>Clear</button>
    </div>
  </div>

  <div class="console-output" bind:this={outputEl}>
    {#if history.length === 0}
      <div class="console-welcome">
        <p class="text-muted">Type Redis commands below. Examples:</p>
        <div class="suggestions">
          {#each suggestions as s}
            <button class="suggestion" on:click={() => { inputValue = s.cmd; }}>
              <span class="suggestion-cmd">{s.cmd}</span>
              <span class="suggestion-desc">{s.desc}</span>
            </button>
          {/each}
        </div>
      </div>
    {:else if filteredHistory.length === 0 && consoleFilter.trim()}
      <p class="text-muted" style="padding: 16px; font-size: 12px; text-align: center;">
        No command outputs matching "{consoleFilter}"
      </p>
    {/if}

    {#each filteredHistory as entry (entry.timestamp)}
      <div class="history-entry">
        <div class="entry-command">
          <span class="prompt">redis&gt;</span>
          <span class="cmd-text">{entry.command}</span>
        </div>
        <pre class="entry-result" class:error={entry.isError}>{entry.result}</pre>
      </div>
    {/each}

    {#if executing}
      <div class="executing">
        <span class="animate-spin" style="display: flex;"><Icons name="refresh" size={13} /></span>
        <span>Executing...</span>
      </div>
    {/if}
  </div>

  <div class="console-input">
    <span class="prompt-input">redis&gt;</span>
    <input
      class="input cmd-input"
      bind:value={inputValue}
      on:keydown={handleKeyDown}
      placeholder="Type a Redis command..."
      disabled={executing}
    />
    <button class="btn btn-sm btn-primary" on:click={executeCommand} disabled={executing || !inputValue.trim()} title="Execute command (Enter)">
      <Icons name="play" size={11} />
    </button>
  </div>
</div>

<style>
  .console {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }
  .console-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--gap-sm) var(--gap-md);
    border-bottom: 1px solid var(--border-primary);
  }
  .console-actions {
    display: flex;
    align-items: center;
    gap: var(--gap-sm);
  }
  .console-filter-wrap {
    position: relative;
    display: flex;
    align-items: center;
  }
  .console-filter-input {
    width: 170px;
    height: 26px;
    font-size: 11px;
    padding-right: 22px;
    background: var(--bg-primary);
    border: 1px solid var(--border-secondary);
    border-radius: var(--radius-sm);
    color: var(--text-primary);
  }
  .clear-mini-btn {
    position: absolute;
    right: 4px;
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 10px;
    cursor: pointer;
    padding: 2px 4px;
  }
  .clear-mini-btn:hover {
    color: var(--text-primary);
  }
  .console-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .console-output {
    flex: 1;
    overflow-y: auto;
    padding: var(--gap-md);
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.5;
  }

  .console-welcome {
    padding: var(--gap-md) 0;
  }
  .console-welcome p {
    margin-bottom: var(--gap-md);
    font-family: var(--font-sans);
  }
  .suggestions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--gap-xs);
  }
  .suggestion {
    background: var(--bg-tertiary);
    border: 1px solid var(--border-primary);
    border-radius: var(--radius-sm);
    padding: 4px 8px;
    cursor: pointer;
    font-family: var(--font-mono);
    font-size: 11px;
    display: flex;
    gap: var(--gap-sm);
    transition: all var(--transition-fast);
    color: var(--text-primary);
  }
  .suggestion:hover {
    border-color: var(--accent);
    background: var(--bg-hover);
  }
  .suggestion-cmd { color: var(--accent); }
  .suggestion-desc { color: var(--text-muted); font-family: var(--font-sans); }

  .history-entry {
    margin-bottom: var(--gap-md);
  }
  .entry-command {
    display: flex;
    gap: var(--gap-sm);
    color: var(--text-primary);
  }
  .prompt, .prompt-input {
    color: var(--success);
    font-weight: 600;
    user-select: none;
    flex-shrink: 0;
  }
  .cmd-text {
    color: var(--accent);
  }
  .entry-result {
    margin: 2px 0 0 0;
    padding: 0;
    white-space: pre-wrap;
    word-break: break-all;
    color: var(--text-secondary);
    font-family: var(--font-mono);
    font-size: 12px;
  }
  .entry-result.error {
    color: var(--error);
  }

  .executing {
    color: var(--text-muted);
    font-family: var(--font-sans);
    font-size: 12px;
    display: flex;
    align-items: center;
    gap: var(--gap-sm);
  }

  .console-input {
    display: flex;
    align-items: center;
    gap: var(--gap-sm);
    padding: var(--gap-sm) var(--gap-md);
    border-top: 1px solid var(--border-primary);
    background: var(--bg-tertiary);
  }
  .cmd-input {
    flex: 1;
    font-family: var(--font-mono);
    font-size: 12px;
    background: transparent;
    border: none;
    padding: var(--gap-xs) 0;
  }
  .cmd-input:focus {
    outline: none;
    box-shadow: none;
    border: none;
  }
</style>
