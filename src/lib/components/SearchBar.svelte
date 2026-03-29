<script lang="ts">
  import { searchPattern, loadKeys, isLoading } from '$lib/stores/redis';

  let inputValue = '';
  let debounceTimer: ReturnType<typeof setTimeout>;

  function handleInput() {
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      const pattern = inputValue.trim() || '*';
      loadKeys(pattern, true);
    }, 300);
  }

  function handleClear() {
    inputValue = '';
    loadKeys('*', true);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      clearTimeout(debounceTimer);
      const pattern = inputValue.trim() || '*';
      loadKeys(pattern, true);
    }
  }
</script>

<div class="search-bar">
  <div class="search-icon">
    {#if $isLoading}
      <span class="animate-spin">⟳</span>
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
    placeholder="Search keys... (e.g. user:* or session:*)"
    spellcheck="false"
  />
  {#if inputValue}
    <button class="clear-btn" on:click={handleClear} title="Clear search">✕</button>
  {/if}
</div>

<style>
  .search-bar {
    display: flex;
    align-items: center;
    padding: 0 var(--gap-md);
    background: var(--bg-primary);
    border: 1px solid var(--border-primary);
    border-radius: var(--radius-sm);
    transition: border-color var(--transition-fast);
  }
  .search-bar:focus-within {
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
    padding: 8px 0;
    background: transparent;
    border: none;
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: 12px;
    outline: none;
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
</style>
