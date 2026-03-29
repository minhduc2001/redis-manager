<script lang="ts">
  import {
    keyDetail,
    isLoadingDetail,
    selectedKey,
    updateKeyValue,
    updateHashField,
    removeHashField,
    updateKeyTtl,
    renameRedisKey,
    addListItem,
    addSetMember,
    deleteSelectedKeys,
    loadKeyDetail,
  } from '$lib/stores/redis';

  let editing = false;
  let editValue = '';
  let editTtl = '';
  let renamingKey = false;
  let newKeyName = '';
  let newHashField = '';
  let newHashValue = '';
  let newListItem = '';
  let newSetMember = '';
  let editingHashField: string | null = null;
  let editingHashValue = '';
  let showDeleteConfirm = false;
  let jsonFormatted = true;

  function tryFormatJson(str: string): { isJson: boolean; formatted: string; highlighted: string } {
    try {
      const parsed = JSON.parse(str);
      const formatted = JSON.stringify(parsed, null, 2);
      return { isJson: true, formatted, highlighted: highlightJson(formatted) };
    } catch {
      return { isJson: false, formatted: str, highlighted: str };
    }
  }

  function highlightJson(json: string): string {
    return json.replace(
      /("(?:[^"\\]|\\.)*")(\s*:)?|(\b(?:true|false)\b)|(\bnull\b)|(-?\d+\.?\d*(?:[eE][+-]?\d+)?)|([{}\[\]])|([,:])/g,
      (match, str, colon, bool, nil, num, bracket, punct) => {
        if (str) {
          const escaped = str.replace(/</g, '&lt;').replace(/>/g, '&gt;');
          if (colon) return `<span class="json-key">${escaped}</span>${colon}`;
          return `<span class="json-str">${escaped}</span>`;
        }
        if (bool) return `<span class="json-bool">${bool}</span>`;
        if (nil) return `<span class="json-null">${nil}</span>`;
        if (num) return `<span class="json-num">${num}</span>`;
        if (bracket) return `<span class="json-bracket">${bracket}</span>`;
        if (punct) return `<span class="json-punct">${punct}</span>`;
        return match;
      }
    );
  }

  $: jsonResult = $keyDetail?.value.type === 'String' ? tryFormatJson($keyDetail.value.data) : null;

  $: if ($keyDetail) {
    editing = false;
    renamingKey = false;
    editingHashField = null;
    if ($keyDetail.value.type === 'String') {
      editValue = $keyDetail.value.data;
    }
    editTtl = $keyDetail.ttl >= 0 ? String($keyDetail.ttl) : '';
  }

  function startEdit() {
    if ($keyDetail?.value.type === 'String') {
      editValue = $keyDetail.value.data;
      editing = true;
    }
  }

  async function saveEdit() {
    if ($keyDetail) {
      const ttl = editTtl.trim() ? parseInt(editTtl) : undefined;
      await updateKeyValue($keyDetail.key, editValue, ttl);
      editing = false;
    }
  }

  function cancelEdit() {
    editing = false;
    if ($keyDetail?.value.type === 'String') {
      editValue = $keyDetail.value.data;
    }
  }

  function startRename() {
    if ($keyDetail) {
      newKeyName = $keyDetail.key;
      renamingKey = true;
    }
  }

  async function saveRename() {
    if ($keyDetail && newKeyName.trim() && newKeyName !== $keyDetail.key) {
      await renameRedisKey($keyDetail.key, newKeyName.trim());
      renamingKey = false;
    }
  }

  async function saveTtl() {
    if ($keyDetail) {
      const ttl = editTtl.trim() ? parseInt(editTtl) : -1;
      await updateKeyTtl($keyDetail.key, ttl);
    }
  }

  function askDeleteKey() {
    showDeleteConfirm = true;
  }

  async function doDeleteKey() {
    if ($keyDetail) {
      await deleteSelectedKeys([$keyDetail.key]);
    }
    showDeleteConfirm = false;
  }

  function startEditHashField(field: string, value: string) {
    editingHashField = field;
    editingHashValue = value;
  }

  async function saveHashFieldEdit() {
    if ($keyDetail && editingHashField !== null) {
      await updateHashField($keyDetail.key, editingHashField, editingHashValue);
      editingHashField = null;
    }
  }

  async function handleDeleteHashField(field: string) {
    if ($keyDetail) {
      await removeHashField($keyDetail.key, field);
    }
  }

  async function handleAddHashField() {
    if ($keyDetail && newHashField.trim()) {
      await updateHashField($keyDetail.key, newHashField.trim(), newHashValue);
      newHashField = '';
      newHashValue = '';
    }
  }

  async function handleAddListItem() {
    if ($keyDetail && newListItem.trim()) {
      await addListItem($keyDetail.key, newListItem.trim());
      newListItem = '';
    }
  }

  async function handleAddSetMember() {
    if ($keyDetail && newSetMember.trim()) {
      await addSetMember($keyDetail.key, newSetMember.trim());
      newSetMember = '';
    }
  }

  function formatTtl(seconds: number): string {
    if (seconds < 0) return 'No expiry';
    if (seconds < 60) return `${seconds}s`;
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
    if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`;
    return `${Math.floor(seconds / 86400)}d ${Math.floor((seconds % 86400) / 3600)}h`;
  }

  function getTypeBadgeClass(type: string): string {
    const t = type.toLowerCase();
    const classes: Record<string, string> = {
      string: 'badge-string',
      hash: 'badge-hash',
      list: 'badge-list',
      set: 'badge-set',
      zset: 'badge-zset',
    };
    return classes[t] || 'badge-unknown';
  }
</script>

<div class="key-detail">
  {#if $isLoadingDetail}
    <div class="loading-state">
      <span class="animate-spin" style="font-size: 24px;">⟳</span>
      <p class="text-muted">Loading key details...</p>
    </div>
  {:else if !$keyDetail}
    <div class="empty-state">
      <div class="empty-icon">
        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="var(--text-muted)" stroke-width="1.5">
          <rect x="3" y="3" width="18" height="18" rx="3"/>
          <path d="M9 9h6M9 13h4"/>
        </svg>
      </div>
      <p>Select a key to view details</p>
      <p class="text-muted">Click on a key from the list</p>
    </div>
  {:else}
    <div class="detail-content animate-fade">
      <!-- Header -->
      <div class="detail-header">
        <div class="key-info">
          {#if renamingKey}
            <div class="rename-row">
              <input class="input input-mono" bind:value={newKeyName} on:keydown={(e) => e.key === 'Enter' && saveRename()} />
              <button class="btn btn-sm btn-primary" on:click={saveRename}>Save</button>
              <button class="btn btn-sm" on:click={() => renamingKey = false}>Cancel</button>
            </div>
          {:else}
            <h3 class="key-title mono truncate" title={$keyDetail.key}>{$keyDetail.key}</h3>
          {/if}
          <div class="key-meta">
            <span class="badge {getTypeBadgeClass($keyDetail.key_type)}">{$keyDetail.key_type}</span>
            <span class="meta-item" title="Size">{$keyDetail.size} {$keyDetail.key_type === 'string' ? 'bytes' : 'items'}</span>
            <span class="meta-item" title="TTL">⏱ {formatTtl($keyDetail.ttl)}</span>
          </div>
        </div>
        <div class="detail-actions">
          <button class="btn btn-sm" on:click={startRename} title="Rename key">✏ Rename</button>
          <button class="btn btn-sm" on:click={() => loadKeyDetail($keyDetail.key)} title="Refresh">⟳</button>
          <button class="btn btn-sm btn-danger" on:click={askDeleteKey} title="Delete key">🗑</button>
        </div>
      </div>

      <!-- TTL Editor -->
      <div class="ttl-row">
        <label class="text-muted">TTL (seconds):</label>
        <input class="input input-mono" style="width: 140px;" bind:value={editTtl} placeholder="No expiry" />
        <button class="btn btn-sm" on:click={saveTtl}>Set TTL</button>
      </div>

      <!-- Value Display -->
      <div class="value-section">
        {#if $keyDetail.value.type === 'String'}
          <div class="value-header">
            <span class="text-muted">Value</span>
            {#if editing}
              <div class="value-actions">
                <button class="btn btn-sm btn-primary" on:click={saveEdit}>💾 Save</button>
                <button class="btn btn-sm" on:click={cancelEdit}>Cancel</button>
              </div>
            {:else}
              <button class="btn btn-sm" on:click={startEdit}>✏ Edit</button>
            {/if}
          </div>
          {#if editing}
            <textarea class="value-editor mono" bind:value={editValue} rows="10"></textarea>
          {:else if jsonResult?.isJson}
            <div class="json-toolbar">
              <span class="badge badge-string">JSON</span>
              <button class="btn btn-sm" on:click={() => jsonFormatted = !jsonFormatted}>
                {jsonFormatted ? 'Raw' : 'Formatted'}
              </button>
            </div>
            {#if jsonFormatted}
              <pre class="value-display mono json-display">{@html jsonResult.highlighted}</pre>
            {:else}
              <pre class="value-display mono">{$keyDetail.value.data}</pre>
            {/if}
          {:else}
            <pre class="value-display mono">{$keyDetail.value.data}</pre>
          {/if}

        {:else if $keyDetail.value.type === 'Hash'}
          <div class="value-header">
            <span class="text-muted">Hash Fields ({$keyDetail.size})</span>
          </div>
          <div class="table-container">
            <table>
              <thead>
                <tr>
                  <th>Field</th>
                  <th>Value</th>
                  <th style="width: 80px;">Actions</th>
                </tr>
              </thead>
              <tbody>
                {#each $keyDetail.value.data as item}
                  <tr>
                    <td class="mono">{item.field}</td>
                    <td class="mono">
                      {#if editingHashField === item.field}
                        <input class="input input-mono" bind:value={editingHashValue} on:keydown={(e) => e.key === 'Enter' && saveHashFieldEdit()} />
                      {:else}
                        <span class="cell-value" title={item.value}>{item.value}</span>
                      {/if}
                    </td>
                    <td>
                      {#if editingHashField === item.field}
                        <button class="btn btn-sm btn-primary" on:click={saveHashFieldEdit}>✓</button>
                      {:else}
                        <button class="btn btn-sm btn-icon" on:click={() => startEditHashField(item.field, item.value)}>✏</button>
                        <button class="btn btn-sm btn-icon btn-danger" on:click={() => handleDeleteHashField(item.field)}>✕</button>
                      {/if}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
          <div class="add-row">
            <input class="input input-mono" bind:value={newHashField} placeholder="Field name" />
            <input class="input input-mono" bind:value={newHashValue} placeholder="Value" />
            <button class="btn btn-sm btn-primary" on:click={handleAddHashField}>+ Add</button>
          </div>

        {:else if $keyDetail.value.type === 'List'}
          <div class="value-header">
            <span class="text-muted">List Items ({$keyDetail.size})</span>
          </div>
          <div class="list-items">
            {#each $keyDetail.value.data as item, i}
              <div class="list-item">
                <span class="item-index">{i}</span>
                <span class="mono truncate">{item}</span>
              </div>
            {/each}
          </div>
          <div class="add-row">
            <input class="input input-mono" bind:value={newListItem} placeholder="New item value" on:keydown={(e) => e.key === 'Enter' && handleAddListItem()} />
            <button class="btn btn-sm btn-primary" on:click={handleAddListItem}>+ Add</button>
          </div>

        {:else if $keyDetail.value.type === 'Set'}
          <div class="value-header">
            <span class="text-muted">Set Members ({$keyDetail.size})</span>
          </div>
          <div class="list-items">
            {#each $keyDetail.value.data as item}
              <div class="list-item">
                <span class="mono truncate">{item}</span>
              </div>
            {/each}
          </div>
          <div class="add-row">
            <input class="input input-mono" bind:value={newSetMember} placeholder="New member" on:keydown={(e) => e.key === 'Enter' && handleAddSetMember()} />
            <button class="btn btn-sm btn-primary" on:click={handleAddSetMember}>+ Add</button>
          </div>

        {:else if $keyDetail.value.type === 'ZSet'}
          <div class="value-header">
            <span class="text-muted">Sorted Set ({$keyDetail.size})</span>
          </div>
          <div class="table-container">
            <table>
              <thead>
                <tr>
                  <th>Score</th>
                  <th>Member</th>
                </tr>
              </thead>
              <tbody>
                {#each $keyDetail.value.data as item}
                  <tr>
                    <td class="mono text-accent">{item.score}</td>
                    <td class="mono">{item.member}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>

        {:else}
          <pre class="value-display mono">{$keyDetail.value.data}</pre>
        {/if}
      </div>
    </div>
  {/if}

  {#if showDeleteConfirm}
    <div class="confirm-overlay" on:click={() => showDeleteConfirm = false}>
      <div class="confirm-modal animate-fade" on:click|stopPropagation>
        <p>Delete key <strong class="mono">{$keyDetail?.key}</strong>?</p>
        <p class="text-muted" style="font-size: 11px; margin-top: 4px;">This action cannot be undone.</p>
        <div class="confirm-actions">
          <button class="btn btn-sm" on:click={() => showDeleteConfirm = false}>Cancel</button>
          <button class="btn btn-sm btn-danger" on:click={doDeleteKey}>Delete</button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .key-detail {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .loading-state, .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: var(--gap-sm);
    color: var(--text-secondary);
  }
  .empty-icon {
    opacity: 0.3;
    margin-bottom: var(--gap-md);
  }

  .detail-content {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .detail-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    padding: var(--gap-lg);
    border-bottom: 1px solid var(--border-primary);
    gap: var(--gap-md);
  }
  .key-info {
    flex: 1;
    min-width: 0;
  }
  .key-title {
    font-size: 14px;
    font-weight: 600;
    margin-bottom: var(--gap-sm);
    word-break: break-all;
  }
  .key-meta {
    display: flex;
    align-items: center;
    gap: var(--gap-md);
    font-size: 12px;
  }
  .meta-item {
    color: var(--text-secondary);
  }
  .detail-actions {
    display: flex;
    gap: var(--gap-xs);
    flex-shrink: 0;
  }

  .rename-row {
    display: flex;
    gap: var(--gap-sm);
    align-items: center;
    margin-bottom: var(--gap-sm);
  }

  .ttl-row {
    display: flex;
    align-items: center;
    gap: var(--gap-sm);
    padding: var(--gap-sm) var(--gap-lg);
    border-bottom: 1px solid var(--border-primary);
    font-size: 12px;
  }

  .value-section {
    flex: 1;
    overflow-y: auto;
    padding: var(--gap-lg);
  }
  .value-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--gap-md);
    font-size: 12px;
    font-weight: 500;
  }
  .value-actions {
    display: flex;
    gap: var(--gap-sm);
  }

  .value-display {
    padding: var(--gap-md);
    background: var(--bg-primary);
    border: 1px solid var(--border-primary);
    border-radius: var(--radius-sm);
    white-space: pre-wrap;
    word-break: break-all;
    max-height: 400px;
    overflow-y: auto;
    font-size: 12px;
    line-height: 1.6;
  }

  .json-toolbar {
    display: flex;
    align-items: center;
    gap: var(--gap-sm);
    margin-bottom: var(--gap-sm);
  }
  .json-display {
    tab-size: 2;
    -moz-tab-size: 2;
  }
  .json-display :global(.json-key) { color: #00d4ff; }
  .json-display :global(.json-str) { color: #00e676; }
  .json-display :global(.json-num) { color: #ffab40; }
  .json-display :global(.json-bool) { color: #ce93d8; }
  .json-display :global(.json-null) { color: #ff5252; font-style: italic; }
  .json-display :global(.json-bracket) { color: #78909c; font-weight: 600; }
  .json-display :global(.json-punct) { color: #546e7a; }
  .value-editor {
    width: 100%;
    padding: var(--gap-md);
    background: var(--bg-primary);
    border: 1px solid var(--border-accent);
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.6;
    resize: vertical;
    outline: none;
  }

  /* Table */
  .table-container {
    overflow: auto;
    max-height: 400px;
    border: 1px solid var(--border-primary);
    border-radius: var(--radius-sm);
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
  }
  th {
    position: sticky;
    top: 0;
    background: var(--bg-tertiary);
    padding: var(--gap-sm) var(--gap-md);
    text-align: left;
    font-weight: 500;
    color: var(--text-secondary);
    border-bottom: 1px solid var(--border-primary);
  }
  td {
    padding: var(--gap-xs) var(--gap-md);
    border-bottom: 1px solid var(--border-primary);
    max-width: 300px;
  }
  .cell-value {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 250px;
  }
  tr:hover {
    background: var(--bg-hover);
  }

  /* List items */
  .list-items {
    border: 1px solid var(--border-primary);
    border-radius: var(--radius-sm);
    max-height: 400px;
    overflow-y: auto;
  }
  .list-item {
    display: flex;
    align-items: center;
    gap: var(--gap-md);
    padding: var(--gap-xs) var(--gap-md);
    border-bottom: 1px solid var(--border-primary);
    font-size: 12px;
  }
  .list-item:last-child {
    border-bottom: none;
  }
  .list-item:hover {
    background: var(--bg-hover);
  }
  .item-index {
    color: var(--text-muted);
    font-size: 10px;
    min-width: 30px;
    font-family: var(--font-mono);
  }

  .add-row {
    display: flex;
    gap: var(--gap-sm);
    margin-top: var(--gap-md);
  }
  .add-row .input {
    flex: 1;
  }

  /* Confirm modal */
  .confirm-overlay {
    position: fixed; inset: 0;
    background: rgba(0,0,0,0.5);
    display: flex; align-items: center; justify-content: center;
    z-index: 1000;
  }
  .confirm-modal {
    background: var(--bg-secondary);
    border: 1px solid var(--border-secondary);
    border-radius: var(--radius-lg);
    padding: var(--gap-lg);
    min-width: 320px;
    box-shadow: var(--shadow-lg);
  }
  .confirm-modal p { font-size: 13px; margin-bottom: var(--gap-md); }
  .confirm-actions {
    display: flex; gap: var(--gap-sm); justify-content: flex-end;
  }
</style>
