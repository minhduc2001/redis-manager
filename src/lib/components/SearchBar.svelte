<script lang="ts">
  import { searchPattern, searchMode, loadKeys, searchKeys, isLoading, isSearching } from '$lib/stores/redis';

  let inputValue = '';
  let debounceTimer: ReturnType<typeof setTimeout>;
  let currentMode: 'exact' | 'like' = 'exact';

  // Sync mode with store
  searchMode.subscribe((v) => (currentMode = v));

  function handleInput() {
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      doSearch();
    }, 400);
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
    }
  }

  function toggleMode() {
    currentMode = currentMode === 'exact' ? 'like' : 'exact';
    searchMode.set(currentMode);
    // Re-search if there's a value
    if (inputValue.trim()) {
      clearTimeout(debounceTimer);
      doSearch();
    }
  }
</script>

<div class="search-bar">
  <button
    class="mode-toggle"
    class:mode-exact={currentMode === 'exact'}
    class:mode-like={currentMode === 'like'}
    on:click={toggleMode}
    title={currentMode === 'exact' ? 'Exact match — click to switch to Like (wildcard)' : 'Like search (substring) — click to switch to Exact'}
  >
    {#if currentMode === 'exact'}
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M4 7h16M4 12h10M4 17h6"/>
      </svg>
      <span>Exact</span>
    {:else}
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M3 12h4l3-9 4 18 3-9h4"/>
      </svg>
      <span>Like</span>
    {/if}
  </button>

  <div class="search-input-wrap">
    <div class="search-icon">
      {#if $isSearching}
        <span class="animate-spin">⟳</span>
      {:else}
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="11" cy="11" r="8"/><path d="m21 21-4.35-4.35"/>
        </svg>
      {/if}
    </div>
    <input
      class="search-input"
      bind:value={inputValue}
      on:input={handleInput}
      on:keydown={handleKeydown}
      placeholder={currentMode === 'exact' ? 'Enter exact key name...' : 'Search keys... (e.g. user, session)'}
      spellcheck="false"
    />
    {#if inputValue}
      <button class="clear-btn" on:click={handleClear} title="Clear search">✕</button>
    {/if}
  </div>
</div>

{#if $isSearching}
  <div class="search-status">
    <span class="animate-pulse">🔍</span>
    <span>Scanning database...</span>
  </div>
{/if}

<style>
  .search-bar {
    display: flex;
    align-items: center;
    gap: var(--gap-xs);
  }

  .mode-toggle {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 6px 10px;
    border: 1px solid var(--border-secondary);
    border-radius: var(--radius-sm);
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    font-size: 10px;
    font-weight: 600;
    font-family: var(--font-sans);
    cursor: pointer;
    transition: all var(--transition-fast);
    white-space: nowrap;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    flex-shrink: 0;
  }
  .mode-toggle:hover {
    background: var(--bg-hover);
    border-color: var(--border-accent);
  }
  .mode-toggle.mode-exact {
    color: var(--type-string);
    border-color: rgba(79, 195, 247, 0.3);
    background: rgba(79, 195, 247, 0.08);
  }
  .mode-toggle.mode-like {
    color: var(--type-hash);
    border-color: rgba(171, 71, 188, 0.3);
    background: rgba(171, 71, 188, 0.08);
  }

  .search-input-wrap {
    display: flex;
    align-items: center;
    flex: 1;
    padding: 0 var(--gap-md);
    background: var(--bg-primary);
    border: 1px solid var(--border-primary);
    border-radius: var(--radius-sm);
    transition: border-color var(--transition-fast);
  }
  .search-input-wrap:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-glow);
  }

  .search-icon {
    color: var(--text-muted);
    display: flex;
    margin-right: var(--gap-sm);
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
  }

  .clear-btn {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 4px;
    font-size: 12px;
    display: flex;
    border-radius: 50%;
    transition: all var(--transition-fast);
  }
  .clear-btn:hover {
    color: var(--text-primary);
    background: var(--bg-hover);
  }

  .search-status {
    display: flex;
    align-items: center;
    gap: var(--gap-sm);
    padding: 4px var(--gap-md);
    font-size: 11px;
    color: var(--text-accent);
    background: rgba(0, 212, 255, 0.04);
    border-top: 1px solid var(--border-primary);
  }
</style>
