<script lang="ts">
  import { searchPattern, searchMode, loadKeys, searchKeys, isLoading, isSearching } from '$lib/stores/redis';
  import type { SearchMode } from '$lib/types';

  let inputValue = '';
  let debounceTimer: ReturnType<typeof setTimeout>;
  let currentMode: SearchMode = 'contains';

  // Sync mode with store
  searchMode.subscribe((v) => (currentMode = v));

  // Sync inputValue when searchPattern or searchMode changes externally
  let lastSyncedPattern = '';
  $: if ($searchPattern !== lastSyncedPattern) {
    lastSyncedPattern = $searchPattern;
    if ($searchPattern === '*') {
      inputValue = '';
    } else if ($searchMode === 'prefix' && $searchPattern.endsWith('*')) {
      inputValue = $searchPattern.slice(0, -1);
    } else if ($searchMode === 'contains' && $searchPattern.startsWith('*') && $searchPattern.endsWith('*') && $searchPattern.length > 2) {
      inputValue = $searchPattern.slice(1, -1);
    } else {
      inputValue = $searchPattern;
    }
  }

  function handleInput() {
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      doSearch();
    }, 350);
  }

  function doSearch() {
    const val = inputValue.trim();
    if (!val) {
      // Empty search — reload all keys (browse mode)
      loadKeys('*', true);
      return;
    }
    searchKeys(val, currentMode);
  }

  function handleClear() {
    inputValue = '';
    loadKeys('*', true);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      clearTimeout(debounceTimer);
      doSearch();
    } else if (e.key === 'Escape') {
      handleClear();
    }
  }

  function setMode(mode: SearchMode) {
    if (currentMode === mode) return;
    currentMode = mode;
    searchMode.set(mode);
    if (inputValue.trim()) {
      clearTimeout(debounceTimer);
      doSearch();
    }
  }

  $: placeholderText = currentMode === 'contains'
    ? 'Search substring (e.g. user, token)...'
    : currentMode === 'prefix'
    ? 'Search prefix (e.g. user:, cache:)...'
    : 'Exact key name (e.g. config:main)...';
</script>

<div class="search-container">
  <div class="search-top-bar">
    <div class="mode-tabs">
      <button
        type="button"
        class="mode-tab"
        class:active={currentMode === 'contains'}
        on:click={() => setMode('contains')}
        title="Contains mode: matches *keyword* anywhere in key"
      >
        <span>*Contains*</span>
      </button>
      <button
        type="button"
        class="mode-tab"
        class:active={currentMode === 'prefix'}
        on:click={() => setMode('prefix')}
        title="Prefix mode: matches keyword* at start of key"
      >
        <span>Prefix*</span>
      </button>
      <button
        type="button"
        class="mode-tab"
        class:active={currentMode === 'exact'}
        on:click={() => setMode('exact')}
        title="Exact mode: matches exact key name"
      >
        <span>Exact</span>
      </button>
    </div>
  </div>

  <div class="search-input-wrap">
    <div class="search-icon">
      {#if $isSearching}
        <span class="animate-spin text-accent">⟳</span>
      {:else}
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="11" cy="11" r="8"/><path d="m21 21-4.35-4.35"/>
        </svg>
      {/if}
    </div>

    <input
      class="search-input"
      bind:value={inputValue}
      on:input={handleInput}
      on:keydown={handleKeydown}
      placeholder={placeholderText}
      spellcheck="false"
    />

    {#if inputValue}
      <button class="clear-btn" on:click={handleClear} title="Clear search (Esc)">✕</button>
    {/if}

    <button
      class="btn-search-go"
      on:click={doSearch}
      title="Search (Enter)"
      disabled={$isSearching}
    >
      ↵
    </button>
  </div>
</div>

{#if $isSearching}
  <div class="search-status animate-fade">
    <span class="pulse-dot"></span>
    <span>Scanning cluster & nodes for matches...</span>
  </div>
{/if}

<style>
  .search-container {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .search-top-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .mode-tabs {
    display: flex;
    background: var(--bg-primary);
    border: 1px solid var(--border-primary);
    border-radius: var(--radius-sm);
    padding: 2px;
    gap: 2px;
    width: 100%;
  }

  .mode-tab {
    flex: 1;
    background: none;
    border: none;
    color: var(--text-muted);
    font-size: 10px;
    font-weight: 600;
    padding: 3px 6px;
    border-radius: 4px;
    cursor: pointer;
    transition: all var(--transition-fast);
    text-align: center;
    font-family: var(--font-sans);
  }

  .mode-tab:hover:not(.active) {
    color: var(--text-primary);
    background: var(--bg-hover);
  }

  .mode-tab.active {
    background: var(--bg-tertiary);
    color: var(--accent);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
  }

  .search-input-wrap {
    display: flex;
    align-items: center;
    padding: 0 4px 0 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border-secondary);
    border-radius: var(--radius-sm);
    transition: all var(--transition-fast);
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.2);
  }

  .search-input-wrap:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-glow), inset 0 1px 2px rgba(0, 0, 0, 0.2);
  }

  .search-icon {
    color: var(--text-muted);
    display: flex;
    align-items: center;
    margin-right: 8px;
    flex-shrink: 0;
  }

  .search-input {
    flex: 1;
    padding: 7px 0;
    background: transparent;
    border: none;
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: 12px;
    outline: none;
    min-width: 0;
  }

  .search-input::placeholder {
    color: var(--text-muted);
    font-family: var(--font-sans);
    font-size: 11px;
  }

  .clear-btn {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 3px 6px;
    font-size: 11px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    transition: all var(--transition-fast);
  }

  .clear-btn:hover {
    color: var(--text-primary);
    background: var(--bg-hover);
  }

  .btn-search-go {
    background: var(--bg-tertiary);
    border: 1px solid var(--border-primary);
    color: var(--text-secondary);
    font-size: 11px;
    padding: 2px 7px;
    border-radius: 4px;
    cursor: pointer;
    margin-left: 4px;
    transition: all var(--transition-fast);
  }

  .btn-search-go:hover:not(:disabled) {
    color: var(--accent);
    border-color: var(--accent);
    background: var(--bg-hover);
  }

  .search-status {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 8px;
    font-size: 11px;
    color: var(--accent);
    background: rgba(0, 212, 255, 0.06);
    border-radius: var(--radius-sm);
    border: 1px solid rgba(0, 212, 255, 0.15);
  }

  .pulse-dot {
    width: 6px;
    height: 6px;
    background: var(--accent);
    border-radius: 50%;
    animation: pulse 1s infinite;
  }
</style>
