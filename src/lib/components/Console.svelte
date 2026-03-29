<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

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

  async function executeCommand() {
    const cmd = inputValue.trim();
    if (!cmd || executing) return;

    commandHistory = [cmd, ...commandHistory.slice(0, 99)];
    historyIndex = -1;
    inputValue = '';
    executing = true;

    try {
      const result = await invoke<string>('execute_command', { command: cmd });
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
    <span class="console-title">⌨ Redis CLI</span>
    <div class="console-actions">
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
    {/if}

    {#each history as entry (entry.timestamp)}
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
        <span class="animate-spin">⟳</span> Executing...
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
    <button class="btn btn-sm btn-primary" on:click={executeCommand} disabled={executing || !inputValue.trim()}>
      ▶
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
