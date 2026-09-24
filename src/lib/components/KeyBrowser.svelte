<script lang="ts">
  import {
    keys,
    selectedKey,
    selectedKeys,
    loadKeys,
    loadKeyDetail,
    deleteSelectedKeys,
    selectAllKeys,
    deselectAllKeys,
    invertSelection,
    selectKeys,
    deselectKeys,
    hasMore,
    isLoading,
    isSearching,
    searchPattern,
  } from '$lib/stores/redis';
  import SearchBar from './SearchBar.svelte';
  import CreateKeyModal from './CreateKeyModal.svelte';
  import Icons from './Icons.svelte';

  let showDeleteConfirm = false;
  let showCreateModal = false;
  let viewMode: 'tree' | 'flat' = 'tree';
  let expandedFolders: Set<string> = new Set();
  let copyFeedback = false;

  // Flat pagination
  let currentPage = 0;
  const PAGE_SIZE = 250;

  // Folder pagination
  const FOLDER_LIMIT = 250;
  let folderLimits: Map<string, number> = new Map();

  let prevPattern = $searchPattern;
  $: if ($searchPattern !== prevPattern) {
    prevPattern = $searchPattern;
    currentPage = 0;
  }

  function getFolderLimit(path: string): number {
    return folderLimits.get(path) || FOLDER_LIMIT;
  }

  function showMoreInFolder(path: string) {
    const current = getFolderLimit(path);
    folderLimits.set(path, current + FOLDER_LIMIT);
    folderLimits = new Map(folderLimits);
  }

  interface FolderGroup {
    prefix: string;
    keys: Array<{ name: string; key_type: string; shortName: string }>;
  }

  function buildGroups(keyList: Array<{ name: string; key_type: string }>): { folders: FolderGroup[]; rootKeys: Array<{ name: string; key_type: string }> } {
    const groups = new Map<string, Array<{ name: string; key_type: string; shortName: string }>>();
    const rootKeys: Array<{ name: string; key_type: string }> = [];

    for (const key of keyList) {
      const lastIdx = Math.max(key.name.lastIndexOf(':'), key.name.lastIndexOf('.'));

      if (lastIdx <= 0) {
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

    const folders: FolderGroup[] = [];
    for (const [prefix, kList] of groups.entries()) {
      if (kList.length === 1) {
        rootKeys.push(kList[0]);
      } else {
        folders.push({ prefix, keys: kList });
      }
    }
    folders.sort((a, b) => a.prefix.localeCompare(b.prefix));

    return { folders, rootKeys };
  }

  $: grouped = buildGroups($keys);
  $: paginatedKeys = $keys.slice(0, (currentPage + 1) * PAGE_SIZE);
  $: displayedCount = Math.min((currentPage + 1) * PAGE_SIZE, $keys.length);

  // Master selection status
  $: isAllSelected = $keys.length > 0 && $selectedKeys.size === $keys.length;
  $: isIndeterminate = $selectedKeys.size > 0 && $selectedKeys.size < $keys.length;

  function toggleMasterSelect() {
    if (isAllSelected) {
      deselectAllKeys();
    } else {
      selectAllKeys();
    }
  }

  function getFolderSelectionState(folderKeys: Array<{ name: string }>): { checked: boolean; indeterminate: boolean } {
    if (folderKeys.length === 0) return { checked: false, indeterminate: false };
    const selectedCount = folderKeys.filter((k) => $selectedKeys.has(k.name)).length;
    return {
      checked: selectedCount === folderKeys.length,
      indeterminate: selectedCount > 0 && selectedCount < folderKeys.length,
    };
  }

  function toggleFolderSelect(folderKeys: Array<{ name: string }>, e: MouseEvent) {
    e.stopPropagation();
    const names = folderKeys.map((k) => k.name);
    const { checked } = getFolderSelectionState(folderKeys);
    if (checked) {
      deselectKeys(names);
    } else {
      selectKeys(names);
    }
  }

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

  async function copySelectedKeyNames() {
    const names = Array.from($selectedKeys).join('\n');
    try {
      await navigator.clipboard.writeText(names);
      copyFeedback = true;
      setTimeout(() => {
        copyFeedback = false;
      }, 1800);
    } catch (err) {
      console.error('Failed to copy', err);
    }
  }

  function confirmDelete() {
    showDeleteConfirm = true;
  }

  async function handleDelete() {
    await deleteSelectedKeys(Array.from($selectedKeys));
    showDeleteConfirm = false;
  }

  async function loadMore() {
    await loadKeys();
    currentPage = Math.max(currentPage, Math.floor(($keys.length - 1) / PAGE_SIZE));
  }

  function showMore() {
    currentPage++;
  }

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
    const classes: Record<string, string> = {
      string: 'badge-string',
      hash: 'badge-hash',
      list: 'badge-list',
      set: 'badge-set',
      zset: 'badge-zset',
    };
    return classes[type] || 'badge-unknown';
  }

  function getTypeIcon(type: string): string {
    const icons: Record<string, string> = {
      string: 'S',
      hash: 'H',
      list: 'L',
      set: '∪',
      zset: 'Z',
    };
    return icons[type] || '?';
  }

  function handleWindowKeydown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'a') {
      const activeEl = document.activeElement;
      if (activeEl && (activeEl.tagName === 'INPUT' || activeEl.tagName === 'TEXTAREA')) {
        return;
      }
      e.preventDefault();
      selectAllKeys();
    }
  }
</script>

<svelte:window on:keydown={handleWindowKeydown} />

<div class="key-browser">
  <!-- Search & Toolbar -->
  <div class="browser-header">
    <SearchBar />

    <div class="header-actions">
      <!-- Master Checkbox & Quick Info -->
      <div class="master-select-row">
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="custom-checkbox-wrap" on:click={toggleMasterSelect} title={isAllSelected ? 'Deselect all' : 'Select all loaded keys'}>
          <div
            class="checkbox"
            class:checked={isAllSelected}
            class:indeterminate={isIndeterminate}
          >
            {#if isAllSelected}✓{:else if isIndeterminate}—{/if}
          </div>
          <span class="select-label">
            {#if $selectedKeys.size > 0}
              <strong>{$selectedKeys.size}</strong>/{$keys.length} selected
            {:else}
              Select all ({$keys.length})
            {/if}
          </span>
        </div>

        <div class="header-right-btns">
          <!-- View toggle -->
          <div class="view-toggle">
            <button
              class="toggle-btn"
              class:active={viewMode === 'tree'}
              on:click={() => setViewMode('tree')}
              title="Tree view"
            >
              <Icons name="tree" size={13} />
              <span>Tree</span>
            </button>
            <button
              class="toggle-btn"
              class:active={viewMode === 'flat'}
              on:click={() => setViewMode('flat')}
              title="Flat view"
            >
              <Icons name="flat" size={13} />
              <span>Flat</span>
            </button>
          </div>

          <!-- Add Key Button -->
          <button class="btn btn-sm btn-accent" on:click={() => showCreateModal = true} title="Create new key">
            <Icons name="plus" size={13} />
            <span>New</span>
          </button>

          <!-- Refresh -->
          <button class="btn btn-sm btn-icon" on:click={handleRefresh} title="Refresh keys (Reload)">
            <span class:animate-spin={$isLoading} style="display: flex;">
              <Icons name="refresh" size={13} />
            </span>
          </button>
        </div>
      </div>

      <!-- Batch Actions Bar (when at least 1 key is selected) -->
      {#if $selectedKeys.size > 0}
        <div class="batch-bar animate-fade">
          <div class="batch-left">
            <button class="batch-link" on:click={selectAllKeys}>All</button>
            <span class="sep">•</span>
            <button class="batch-link" on:click={deselectAllKeys}>None</button>
            <span class="sep">•</span>
            <button class="batch-link" on:click={invertSelection}>Invert</button>
          </div>

          <div class="batch-right">
            <button class="btn btn-sm" on:click={copySelectedKeyNames} title="Copy selected key names to clipboard">
              {#if copyFeedback}
                <Icons name="check" size={13} />
                <span>Copied!</span>
              {:else}
                <Icons name="copy" size={13} />
                <span>Copy</span>
              {/if}
            </button>
            <button class="btn btn-sm btn-danger" on:click={confirmDelete} title="Delete selected keys">
              <Icons name="trash" size={13} />
              <span>Delete ({$selectedKeys.size})</span>
            </button>
          </div>
        </div>
      {/if}
    </div>
  </div>

  <!-- Key List -->
  <div class="key-list">
    {#if $keys.length === 0 && !$isLoading && !$isSearching}
      <div class="empty-state">
        <div class="empty-icon" style="color: var(--text-muted); opacity: 0.6;">
          <Icons name="folder-open" size={42} />
        </div>
        <p class="empty-text">No keys found</p>
        <p class="text-muted">Try a different search or create a new key</p>
        <button class="btn btn-sm btn-accent" style="margin-top: 8px; display: inline-flex; align-items: center; gap: 4px;" on:click={() => showCreateModal = true}>
          <Icons name="plus" size={13} />
          <span>Create Key</span>
        </button>
      </div>
    {:else if viewMode === 'tree'}
      <!-- Tree View -->
      {#each grouped.folders as folder (folder.prefix)}
        {@const folderState = getFolderSelectionState(folder.keys)}
        <div class="tree-folder">
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div class="folder-header" on:click={() => toggleFolder(folder.prefix)}>
            <span class="folder-arrow">
              <Icons name={expandedFolders.has(folder.prefix) ? 'chevron-down' : 'chevron-right'} size={12} />
            </span>

            <!-- Folder Select Checkbox -->
            <div class="folder-checkbox" on:click={(e) => toggleFolderSelect(folder.keys, e)} title="Select all in folder">
              <div
                class="checkbox checkbox-sm"
                class:checked={folderState.checked}
                class:indeterminate={folderState.indeterminate}
              >
                {#if folderState.checked}✓{:else if folderState.indeterminate}—{/if}
              </div>
            </div>

            <span class="folder-icon-sym" style="display: flex; align-items: center;">
              <Icons name={expandedFolders.has(folder.prefix) ? 'folder-open' : 'folder'} size={14} />
            </span>
            <span class="folder-name truncate">{folder.prefix}</span>
            <span class="folder-count">{folder.keys.length}</span>
          </div>

          {#if expandedFolders.has(folder.prefix)}
            <div class="folder-children animate-fade">
              {#each folder.keys.slice(0, folderLimits.get(folder.prefix) || FOLDER_LIMIT) as key (key.name)}
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <div
                  class="key-item indent1"
                  class:selected={$selectedKey === key.name}
                  class:checked={$selectedKeys.has(key.name)}
                  on:click={() => handleKeyClick(key.name)}
                >
                  <div class="key-checkbox" on:click={(e) => toggleSelect(key.name, e)}>
                    <div class="checkbox" class:checked={$selectedKeys.has(key.name)}>
                      {#if $selectedKeys.has(key.name)}✓{/if}
                    </div>
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

      <!-- Root-level keys (no prefix) -->
      {#each grouped.rootKeys as key (key.name)}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="key-item"
          class:selected={$selectedKey === key.name}
          class:checked={$selectedKeys.has(key.name)}
          on:click={() => handleKeyClick(key.name)}
        >
          <div class="key-checkbox" on:click={(e) => toggleSelect(key.name, e)}>
            <div class="checkbox" class:checked={$selectedKeys.has(key.name)}>
              {#if $selectedKeys.has(key.name)}✓{/if}
            </div>
          </div>
          <span class="badge {getTypeBadgeClass(key.key_type)}">{getTypeIcon(key.key_type)}</span>
          <span class="key-name truncate" title={key.name}>{key.name}</span>
        </div>
      {/each}

    {:else}
      <!-- Flat View -->
      {#each paginatedKeys as key (key.name)}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="key-item"
          class:selected={$selectedKey === key.name}
          class:checked={$selectedKeys.has(key.name)}
          on:click={() => handleKeyClick(key.name)}
        >
          <div class="key-checkbox" on:click={(e) => toggleSelect(key.name, e)}>
            <div class="checkbox" class:checked={$selectedKeys.has(key.name)}>
              {#if $selectedKeys.has(key.name)}✓{/if}
            </div>
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
        <button class="btn btn-sm btn-primary" style="display: inline-flex; align-items: center; gap: 6px;" on:click={loadMore} disabled={$isLoading || $isSearching}>
          {#if $isLoading || $isSearching}
            <span class="animate-spin" style="display: flex;"><Icons name="refresh" size={13} /></span>
            <span>Scanning...</span>
          {:else}
            <Icons name="bolt" size={13} />
            <span>Scan more keys from Redis</span>
          {/if}
        </button>
      </div>
    {/if}
  </div>

  <!-- Footer Info -->
  <div class="browser-footer">
    {#if $searchPattern !== '*'}
      <span class="text-accent" style="display: flex; align-items: center; gap: 4px;">
        <Icons name="search" size={12} />
        <span>{$keys.length} matches</span>
      </span>
    {:else}
      <span class="text-muted">{$keys.length} keys loaded</span>
    {/if}
    {#if viewMode === 'flat'}
      <span class="text-muted">• Showing {displayedCount}</span>
    {/if}
    {#if $hasMore}
      <span class="text-accent">• More available in Redis</span>
    {/if}
  </div>

  <!-- Delete Modal -->
  {#if showDeleteConfirm}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="modal-overlay" on:click={() => showDeleteConfirm = false}>
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="modal animate-fade" on:click|stopPropagation>
        <h3>Delete {$selectedKeys.size} Key(s)</h3>
        <p>Are you sure you want to delete these keys from Redis?</p>
        <div class="delete-key-list">
          {#each Array.from($selectedKeys).slice(0, 8) as k}
            <div class="delete-key-item mono truncate">• {k}</div>
          {/each}
          {#if $selectedKeys.size > 8}
            <div class="text-muted" style="font-size: 11px; margin-top: 4px;">
              ...and {$selectedKeys.size - 8} more
            </div>
          {/if}
        </div>
        <p class="text-muted" style="margin-top: 8px; font-size: 11px;">This action cannot be undone.</p>
        <div class="modal-actions">
          <button class="btn" on:click={() => showDeleteConfirm = false}>Cancel</button>
          <button class="btn btn-danger" on:click={handleDelete}>Delete Permanently</button>
        </div>
      </div>
    </div>
  {/if}

  <!-- Create Key Modal -->
  {#if showCreateModal}
    <CreateKeyModal onClose={() => showCreateModal = false} />
  {/if}
</div>

<style>
  .key-browser {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    background: var(--bg-secondary);
  }

  .browser-header {
    padding: var(--gap-md);
    border-bottom: 1px solid var(--border-primary);
    display: flex;
    flex-direction: column;
    gap: var(--gap-sm);
    background: rgba(15, 15, 35, 0.6);
  }

  .header-actions {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .master-select-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--gap-sm);
  }

  .custom-checkbox-wrap {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    user-select: none;
    font-size: 11px;
    color: var(--text-secondary);
    transition: color var(--transition-fast);
  }
  .custom-checkbox-wrap:hover {
    color: var(--text-primary);
  }

  .select-label strong {
    color: var(--accent);
  }

  .header-right-btns {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .view-toggle {
    display: flex;
    border: 1px solid var(--border-secondary);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }
  .toggle-btn {
    background: var(--bg-tertiary);
    border: none;
    padding: 3px 8px;
    cursor: pointer;
    font-size: 11px;
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

  .btn-accent {
    background: rgba(0, 212, 255, 0.15);
    border: 1px solid rgba(0, 212, 255, 0.35);
    color: var(--accent);
    font-weight: 600;
  }
  .btn-accent:hover {
    background: rgba(0, 212, 255, 0.25);
    box-shadow: 0 0 10px var(--accent-glow);
  }

  /* Batch actions toolbar */
  .batch-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 10px;
    background: rgba(0, 212, 255, 0.08);
    border: 1px solid rgba(0, 212, 255, 0.25);
    border-radius: var(--radius-sm);
    gap: 8px;
  }
  .batch-left {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
  }
  .batch-link {
    background: none;
    border: none;
    color: var(--accent);
    font-size: 11px;
    font-weight: 500;
    cursor: pointer;
    padding: 0;
    text-decoration: underline;
  }
  .batch-link:hover {
    color: #fff;
  }
  .sep {
    color: var(--text-muted);
    font-size: 9px;
  }
  .batch-right {
    display: flex;
    gap: 6px;
  }

  /* Checkbox styling */
  .checkbox {
    width: 15px;
    height: 15px;
    border: 1px solid var(--border-secondary);
    border-radius: 3px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 10px;
    font-weight: 700;
    color: var(--accent);
    background: var(--bg-primary);
    transition: all var(--transition-fast);
    flex-shrink: 0;
  }
  .checkbox.checked {
    background: rgba(0, 212, 255, 0.2);
    border-color: var(--accent);
  }
  .checkbox.indeterminate {
    background: rgba(0, 212, 255, 0.15);
    border-color: var(--accent);
    color: var(--accent);
  }
  .checkbox-sm {
    width: 13px;
    height: 13px;
    font-size: 8px;
  }

  /* Key list */
  .key-list {
    flex: 1;
    overflow-y: auto;
    padding: 4px 0;
  }

  /* Tree View */
  .tree-folder {
    user-select: none;
  }
  .folder-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 12px;
    cursor: pointer;
    font-size: 12px;
    font-weight: 500;
    transition: background var(--transition-fast);
  }
  .folder-header:hover {
    background: var(--bg-hover);
  }
  .folder-arrow {
    font-size: 9px;
    color: var(--text-muted);
    transition: transform var(--transition-fast);
    width: 10px;
    display: inline-block;
    text-align: center;
  }
  .folder-arrow.open {
    transform: rotate(90deg);
  }
  .folder-checkbox {
    display: flex;
    align-items: center;
  }
  .folder-icon-sym {
    font-size: 12px;
  }
  .folder-name {
    flex: 1;
    color: var(--text-primary);
  }
  .folder-count {
    font-size: 10px;
    color: var(--text-muted);
    background: rgba(255, 255, 255, 0.05);
    padding: 1px 6px;
    border-radius: 8px;
  }

  /* Key item */
  .key-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 12px;
    cursor: pointer;
    transition: all var(--transition-fast);
    border-left: 2px solid transparent;
  }
  .key-item.indent1 {
    padding-left: 32px;
  }
  .key-item:hover {
    background: var(--bg-hover);
  }
  .key-item.selected {
    background: var(--bg-active);
    border-left-color: var(--accent);
  }
  .key-item.checked {
    background: rgba(0, 212, 255, 0.05);
  }

  .key-checkbox {
    flex-shrink: 0;
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
    gap: 4px;
    text-align: center;
  }
  .empty-icon {
    font-size: 32px;
    opacity: 0.6;
    margin-bottom: 4px;
  }
  .empty-text {
    font-size: 13px;
    font-weight: 500;
  }

  .load-more {
    padding: var(--gap-sm) var(--gap-md);
    display: flex;
    justify-content: center;
  }
  .folder-load-more {
    padding: 4px 0 4px 32px;
    display: flex;
  }
  .folder-load-more .btn {
    font-size: 10px;
    padding: 2px 8px;
    color: var(--accent);
  }

  .browser-footer {
    padding: 6px 12px;
    border-top: 1px solid var(--border-primary);
    font-size: 11px;
    display: flex;
    gap: var(--gap-sm);
    background: rgba(15, 15, 35, 0.5);
  }

  /* Modal */
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(4px);
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
    max-width: 480px;
    width: 90%;
    box-shadow: var(--shadow-lg);
  }
  .modal h3 {
    font-size: 15px;
    margin-bottom: var(--gap-sm);
  }
  .delete-key-list {
    background: var(--bg-primary);
    border: 1px solid var(--border-primary);
    border-radius: var(--radius-sm);
    padding: 8px 12px;
    margin: var(--gap-md) 0;
    max-height: 140px;
    overflow-y: auto;
  }
  .delete-key-item {
    font-size: 11px;
    color: var(--text-secondary);
    padding: 2px 0;
  }
  .modal-actions {
    display: flex;
    gap: var(--gap-md);
    margin-top: var(--gap-lg);
    justify-content: flex-end;
  }
</style>
