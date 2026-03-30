<script lang="ts">
  import {
    keys,
    selectedKey,
    selectedKeys,
    loadKeys,
    loadKeyDetail,
    deleteSelectedKeys,
    hasMore,
    isLoading,
  } from '$lib/stores/redis';
  import SearchBar from './SearchBar.svelte';

  let showDeleteConfirm = false;
  let viewMode: 'tree' | 'flat' = 'tree';
  let expandedFolders: Set<string> = new Set();

  // Flat pagination
  let currentPage = 0;
  const PAGE_SIZE = 50;

  // Folder pagination
  const FOLDER_LIMIT = 50;
  let folderLimits: Map<string, number> = new Map();

  function getFolderLimit(path: string): number {
    return folderLimits.get(path) || FOLDER_LIMIT;
  }

  function showMoreInFolder(path: string) {
    const current = getFolderLimit(path);
    folderLimits.set(path, current + FOLDER_LIMIT);
    folderLimits = new Map(folderLimits);
  }

  // Key delimiter for tree grouping
  const DELIMITERS = /[:.]/;

  interface FolderGroup {
    prefix: string;
    keys: Array<{ name: string; key_type: string; shortName: string }>;
  }

  function buildGroups(keyList: Array<{ name: string; key_type: string }>): { folders: FolderGroup[]; rootKeys: Array<{ name: string; key_type: string }> } {
    const groups = new Map<string, Array<{ name: string; key_type: string; shortName: string }>>();
    const rootKeys: Array<{ name: string; key_type: string }> = [];

    for (const key of keyList) {
      // Find the LAST delimiter position to split prefix and leaf
      const lastIdx = Math.max(key.name.lastIndexOf(':'), key.name.lastIndexOf('.'));

      if (lastIdx <= 0) {
        // No delimiter or at position 0 - root key
        rootKeys.push(key);
        continue;
      }

      const prefix = key.name.substring(0, lastIdx);
      const shortName = key.name.substring(lastIdx + 1);

      if (!groups.has(prefix)) {
        groups.set(prefix, []);
      }
      groups.get(prefix)!.push({ ...key, shortName });
    }

    // Sort folders by name, only create folders for groups with > 1 key
    const folders: FolderGroup[] = [];
    for (const [prefix, keys] of groups.entries()) {
      if (keys.length === 1) {
        // Single key in group - show at root level
        rootKeys.push(keys[0]);
      } else {
        folders.push({ prefix, keys });
      }
    }
    folders.sort((a, b) => a.prefix.localeCompare(b.prefix));

    return { folders, rootKeys };
  }

  $: grouped = buildGroups($keys);
  $: paginatedKeys = $keys.slice(0, (currentPage + 1) * PAGE_SIZE);
  $: totalPages = Math.ceil($keys.length / PAGE_SIZE);
  $: displayedCount = Math.min((currentPage + 1) * PAGE_SIZE, $keys.length);

  function toggleFolder(path: string) {
    if (expandedFolders.has(path)) {
      expandedFolders.delete(path);
    } else {
      expandedFolders.add(path);
    }
    expandedFolders = new Set(expandedFolders);
  }

  function handleKeyClick(keyName: string) {
    selectedKey.set(keyName);
    loadKeyDetail(keyName);
  }

  function toggleSelect(keyName: string, e: MouseEvent) {
    e.stopPropagation();
    selectedKeys.update((set) => {
      const next = new Set(set);
      if (next.has(keyName)) next.delete(keyName);
      else next.add(keyName);
      return next;
    });
  }

  function selectAll() {
    selectedKeys.update(() => new Set($keys.map((k) => k.name)));
  }
  function deselectAll() {
    selectedKeys.set(new Set());
  }

  function confirmDelete() { showDeleteConfirm = true; }
  async function handleDelete() {
    await deleteSelectedKeys(Array.from($selectedKeys));
    showDeleteConfirm = false;
  }

  function loadMore() { loadKeys(); }
  function showMore() { currentPage++; }

  function handleRefresh() {
    currentPage = 0;
    expandedFolders = new Set();
    folderLimits = new Map();
    loadKeys(undefined, true);
  }

  function setViewMode(mode: 'tree' | 'flat') {
    viewMode = mode;
  }

  function getTypeBadgeClass(type: string): string {
    const classes: Record<string, string> = { string: 'badge-string', hash: 'badge-hash', list: 'badge-list', set: 'badge-set', zset: 'badge-zset' };
    return classes[type] || 'badge-unknown';
  }
  function getTypeIcon(type: string): string {
    const icons: Record<string, string> = { string: 'S', hash: 'H', list: 'L', set: '∪', zset: 'Z' };
    return icons[type] || '?';
  }
</script>

<div class="key-browser">
  <div class="browser-header">
    <SearchBar />
    <div class="browser-actions">
      <div class="view-toggle">
        <button class="toggle-btn" class:active={viewMode === 'tree'} on:click={() => setViewMode('tree')} title="Tree view">🗂</button>
        <button class="toggle-btn" class:active={viewMode === 'flat'} on:click={() => setViewMode('flat')} title="Flat view">≡</button>
      </div>
      {#if $selectedKeys.size > 0}
        <span class="selection-count">{$selectedKeys.size} selected</span>
        <button class="btn btn-sm" on:click={deselectAll}>Deselect</button>
        <button class="btn btn-sm btn-danger" on:click={confirmDelete}>🗑 Delete</button>
      {:else}
        <button class="btn btn-sm" on:click={selectAll} disabled={$keys.length === 0}>Select All</button>
      {/if}
      <button class="btn btn-sm" on:click={handleRefresh} title="Refresh">⟳</button>
    </div>
  </div>

  <div class="key-list">
    {#if $keys.length === 0 && !$isLoading}
      <div class="empty-state">
        <div class="empty-icon">📭</div>
        <p>No keys found</p>
        <p class="text-muted">Try a different search pattern</p>
      </div>
    {:else if viewMode === 'tree'}
      <!-- Tree View: single-level folders -->
      {#each grouped.folders as folder (folder.prefix)}
        <div class="tree-folder">
          <div class="folder-header" on:click={() => toggleFolder(folder.prefix)}>
            <span class="folder-icon" class:open={expandedFolders.has(folder.prefix)}>▸</span>
            <span class="folder-name">📁 {folder.prefix}</span>
            <span class="folder-count">{folder.keys.length}</span>
          </div>
          {#if expandedFolders.has(folder.prefix)}
            <div class="folder-children animate-fade">
              {#each folder.keys.slice(0, folderLimits.get(folder.prefix) || FOLDER_LIMIT) as key (key.name)}
                <div class="key-item indent1" class:selected={$selectedKey === key.name} on:click={() => handleKeyClick(key.name)}>
                  <div class="key-checkbox" on:click={(e) => toggleSelect(key.name, e)}>
                    <div class="checkbox" class:checked={$selectedKeys.has(key.name)}>{#if $selectedKeys.has(key.name)}✓{/if}</div>
                  </div>
                  <span class="badge {getTypeBadgeClass(key.key_type)}">{getTypeIcon(key.key_type)}</span>
                  <span class="key-name truncate" title={key.name}>{key.shortName}</span>
                </div>
              {/each}
              {#if folder.keys.length > (folderLimits.get(folder.prefix) || FOLDER_LIMIT)}
                <div class="folder-load-more">
                  <button class="btn btn-sm" on:click={() => showMoreInFolder(folder.prefix)}>
                    Show more ({folderLimits.get(folder.prefix) || FOLDER_LIMIT}/{folder.keys.length})
                  </button>
                </div>
              {/if}
            </div>
          {/if}
        </div>
      {/each}
      <!-- Root-level keys (no prefix or single in group) -->
      {#each grouped.rootKeys as key (key.name)}
        <div class="key-item" class:selected={$selectedKey === key.name} on:click={() => handleKeyClick(key.name)}>
          <div class="key-checkbox" on:click={(e) => toggleSelect(key.name, e)}>
            <div class="checkbox" class:checked={$selectedKeys.has(key.name)}>{#if $selectedKeys.has(key.name)}✓{/if}</div>
          </div>
          <span class="badge {getTypeBadgeClass(key.key_type)}">{getTypeIcon(key.key_type)}</span>
          <span class="key-name truncate" title={key.name}>{key.name}</span>
        </div>
      {/each}
    {:else}
      <!-- Flat View with Pagination -->
      {#each paginatedKeys as key (key.name)}
        <div class="key-item" class:selected={$selectedKey === key.name} class:checked={$selectedKeys.has(key.name)} on:click={() => handleKeyClick(key.name)}>
          <div class="key-checkbox" on:click={(e) => toggleSelect(key.name, e)}>
            <div class="checkbox" class:checked={$selectedKeys.has(key.name)}>{#if $selectedKeys.has(key.name)}✓{/if}</div>
          </div>
          <span class="badge {getTypeBadgeClass(key.key_type)}">{getTypeIcon(key.key_type)}</span>
          <span class="key-name truncate" title={key.name}>{key.name}</span>
        </div>
      {/each}

      {#if displayedCount < $keys.length}
        <div class="load-more">
          <button class="btn btn-sm" on:click={showMore}>
            Show more ({displayedCount}/{$keys.length})
          </button>
        </div>
      {/if}
    {/if}

    {#if $hasMore}
      <div class="load-more">
        <button class="btn btn-sm btn-primary" on:click={loadMore} disabled={$isLoading}>
          {#if $isLoading}
            <span class="animate-spin">⟳</span> Scanning...
          {:else}
            ⚡ Scan more keys from Redis
          {/if}
        </button>
      </div>
    {/if}
  </div>

  <div class="browser-footer">
    <span class="text-muted">{$keys.length} keys loaded</span>
    {#if viewMode === 'flat'}
      <span class="text-muted">• Showing {displayedCount}</span>
    {/if}
    {#if $hasMore}
      <span class="text-accent">• More available</span>
    {/if}
  </div>

  {#if showDeleteConfirm}
    <div class="modal-overlay" on:click={() => showDeleteConfirm = false}>
      <div class="modal animate-fade" on:click|stopPropagation>
        <h3>Delete Keys</h3>
        <p>Are you sure you want to delete <strong>{$selectedKeys.size}</strong> key(s)?</p>
        <p class="text-muted" style="margin-top: 8px; font-size: 12px;">This action cannot be undone.</p>
        <div class="modal-actions">
          <button class="btn" on:click={() => showDeleteConfirm = false}>Cancel</button>
          <button class="btn btn-danger" on:click={handleDelete}>Delete</button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .key-browser {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }
  .browser-header {
    padding: var(--gap-md);
    border-bottom: 1px solid var(--border-primary);
    display: flex;
    flex-direction: column;
    gap: var(--gap-sm);
  }
  .browser-actions {
    display: flex;
    align-items: center;
    gap: var(--gap-sm);
    flex-wrap: wrap;
  }
  .selection-count {
    font-size: 11px;
    color: var(--accent);
    font-weight: 500;
  }

  /* View toggle */
  .view-toggle {
    display: flex;
    border: 1px solid var(--border-secondary);
    border-radius: var(--radius-sm);
    overflow: hidden;
    margin-right: var(--gap-sm);
  }
  .toggle-btn {
    background: var(--bg-tertiary);
    border: none;
    padding: 3px 8px;
    cursor: pointer;
    font-size: 12px;
    color: var(--text-secondary);
    transition: all var(--transition-fast);
  }
  .toggle-btn.active {
    background: var(--bg-active);
    color: var(--accent);
  }
  .toggle-btn:hover:not(.active) {
    background: var(--bg-hover);
  }

  .key-list {
    flex: 1;
    overflow-y: auto;
    padding: var(--gap-xs) 0;
  }

  /* Tree folders */
  .tree-folder, .tree-subfolder {
    user-select: none;
  }
  .folder-header {
    display: flex;
    align-items: center;
    gap: var(--gap-sm);
    padding: 5px var(--gap-md);
    cursor: pointer;
    font-size: 12px;
    font-weight: 500;
    transition: background var(--transition-fast);
  }
  .folder-header:hover {
    background: var(--bg-hover);
  }
  .folder-icon {
    font-size: 10px;
    color: var(--text-muted);
    transition: transform var(--transition-fast);
    width: 12px;
    display: inline-block;
    text-align: center;
  }
  .folder-icon.open {
    transform: rotate(90deg);
  }
  .folder-name {
    flex: 1;
    color: var(--text-primary);
  }
  .folder-count {
    font-size: 10px;
    color: var(--text-muted);
    background: rgba(255,255,255,0.05);
    padding: 1px 6px;
    border-radius: 8px;
  }
  .folder-children {
    /* no extra indent; handled by key-item classes */
  }

  /* Key items */
  .key-item {
    display: flex;
    align-items: center;
    gap: var(--gap-sm);
    padding: 5px var(--gap-md);
    cursor: pointer;
    transition: all var(--transition-fast);
    border-left: 2px solid transparent;
  }
  .key-item.indent1 { padding-left: 28px; }
  .key-item:hover {
    background: var(--bg-hover);
  }
  .key-item.selected {
    background: var(--bg-active);
    border-left-color: var(--accent);
  }
  .key-item.checked {
    background: rgba(0, 212, 255, 0.04);
  }

  .key-checkbox { flex-shrink: 0; }
  .checkbox {
    width: 16px;
    height: 16px;
    border: 1px solid var(--border-secondary);
    border-radius: 3px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 10px;
    color: var(--accent);
    transition: all var(--transition-fast);
  }
  .checkbox.checked {
    background: rgba(0, 212, 255, 0.15);
    border-color: var(--accent);
  }

  .key-name {
    font-family: var(--font-mono);
    font-size: 12px;
    flex: 1;
    min-width: 0;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--gap-2xl);
    gap: var(--gap-sm);
  }
  .empty-icon { font-size: 32px; opacity: 0.5; }

  .load-more {
    padding: var(--gap-sm) var(--gap-md);
    display: flex;
    justify-content: center;
  }
  .folder-load-more {
    padding: 4px 0 4px 28px;
    display: flex;
  }
  .folder-load-more .btn {
    font-size: 10px;
    padding: 2px 8px;
    color: var(--accent);
    border-color: var(--border-primary);
  }

  .browser-footer {
    padding: var(--gap-sm) var(--gap-md);
    border-top: 1px solid var(--border-primary);
    font-size: 11px;
    display: flex;
    gap: var(--gap-sm);
    flex-wrap: wrap;
  }

  /* Modal */
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }
  .modal {
    background: var(--bg-secondary);
    border: 1px solid var(--border-secondary);
    border-radius: var(--radius-lg);
    padding: var(--gap-xl);
    min-width: 360px;
    box-shadow: var(--shadow-lg);
  }
  .modal h3 { font-size: 16px; margin-bottom: var(--gap-md); }
  .modal-actions {
    display: flex;
    gap: var(--gap-md);
    margin-top: var(--gap-xl);
    justify-content: flex-end;
  }
</style>
