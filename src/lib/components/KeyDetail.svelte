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
    deleteListItem,
    addSetMember,
    deleteSetMember,
    addZSetMember,
    deleteZSetMember,
    deleteSelectedKeys,
    loadKeyDetail,
  } from '$lib/stores/redis';
  import type { HashField, ZSetMember } from '$lib/types';
  import Icons from './Icons.svelte';

  // Result search state
  let resultSearchQuery = '';
  let resultSearchInputEl: HTMLInputElement | null = null;

  function escapeHtml(str: string): string {
    return (str || '')
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');
  }

  function highlightText(text: string, query: string): string {
    if (!query || !text) return escapeHtml(text || '');
    const escapedText = escapeHtml(text);
    const trimmed = query.trim();
    if (!trimmed) return escapedText;
    const escapedQuery = escapeHtml(trimmed);
    const regex = new RegExp(`(${escapedQuery.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'gi');
    return escapedText.replace(regex, '<mark class="search-match">$1</mark>');
  }

  function highlightInsideHtml(html: string, query: string): string {
    if (!query || !html) return html || '';
    const trimmed = query.trim();
    if (!trimmed) return html;
    const escapedQuery = escapeHtml(trimmed);
    const regex = new RegExp(`(?![^<]*>)(${escapedQuery.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'gi');
    return html.replace(regex, '<mark class="search-match">$1</mark>');
  }

  function countMatches(text: string, query: string): number {
    if (!query || !text) return 0;
    const trimmed = query.trim();
    if (!trimmed) return 0;
    const regex = new RegExp(trimmed.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'gi');
    const matches = text.match(regex);
    return matches ? matches.length : 0;
  }

  function getSearchPlaceholder(type: string): string {
    switch (type) {
      case 'Hash':
        return 'Filter fields & values (Ctrl+F)...';
      case 'List':
        return 'Filter list items (Ctrl+F)...';
      case 'Set':
        return 'Filter set members (Ctrl+F)...';
      case 'ZSet':
        return 'Filter members & scores (Ctrl+F)...';
      case 'String':
        return 'Search in text / JSON (Ctrl+F)...';
      default:
        return 'Filter in result (Ctrl+F)...';
    }
  }

  function handleSearchKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      resultSearchQuery = '';
      resultSearchInputEl?.blur();
    }
  }

  function handleWindowKeydown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'f') {
      if (resultSearchInputEl) {
        e.preventDefault();
        resultSearchInputEl.focus();
        resultSearchInputEl.select();
      }
    }
  }

  // Reset search when key changes
  $: if ($selectedKey) {
    resultSearchQuery = '';
  }

  // Filtered collections
  $: filteredHashData = ($keyDetail?.value.type === 'Hash' && Array.isArray($keyDetail.value.data))
    ? (resultSearchQuery.trim()
        ? ($keyDetail.value.data as HashField[]).filter((item) =>
            item.field.toLowerCase().includes(resultSearchQuery.trim().toLowerCase()) ||
            item.value.toLowerCase().includes(resultSearchQuery.trim().toLowerCase())
          )
        : ($keyDetail.value.data as HashField[]))
    : [];

  $: filteredListData = ($keyDetail?.value.type === 'List' && Array.isArray($keyDetail.value.data))
    ? ($keyDetail.value.data as string[])
        .map((item, originalIndex) => ({ item, originalIndex }))
        .filter(({ item }) =>
          resultSearchQuery.trim()
            ? item.toLowerCase().includes(resultSearchQuery.trim().toLowerCase())
            : true
        )
    : [];

  $: filteredSetData = ($keyDetail?.value.type === 'Set' && Array.isArray($keyDetail.value.data))
    ? ($keyDetail.value.data as string[])
        .map((item, originalIndex) => ({ item, originalIndex }))
        .filter(({ item }) =>
          resultSearchQuery.trim()
            ? item.toLowerCase().includes(resultSearchQuery.trim().toLowerCase())
            : true
        )
    : [];

  $: filteredZSetData = ($keyDetail?.value.type === 'ZSet' && Array.isArray($keyDetail.value.data))
    ? (resultSearchQuery.trim()
        ? ($keyDetail.value.data as ZSetMember[]).filter((item) =>
            item.member.toLowerCase().includes(resultSearchQuery.trim().toLowerCase()) ||
            String(item.score).toLowerCase().includes(resultSearchQuery.trim().toLowerCase())
          )
        : ($keyDetail.value.data as ZSetMember[]))
    : [];

  $: stringMatchCount = ($keyDetail?.value.type === 'String' && resultSearchQuery.trim())
    ? countMatches($keyDetail.value.data, resultSearchQuery)
    : 0;

  let editing = false;
  let editValue = '';
  let editTtl = '';
  let renamingKey = false;
  let newKeyName = '';
  let newHashField = '';
  let newHashValue = '';
  let newListItem = '';
  let newSetMember = '';
  let newZSetScore = '0';
  let newZSetMember = '';
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

  async function handleDeleteListItem(item: string) {
    if ($keyDetail) {
      await deleteListItem($keyDetail.key, item);
    }
  }

  async function handleDeleteSetMember(member: string) {
    if ($keyDetail) {
      await deleteSetMember($keyDetail.key, member);
    }
  }

  async function handleAddZSetMember() {
    if ($keyDetail && newZSetMember.trim()) {
      const score = parseFloat(newZSetScore.trim()) || 0;
      await addZSetMember($keyDetail.key, score, newZSetMember.trim());
      newZSetMember = '';
      newZSetScore = '0';
    }
  }

  async function handleDeleteZSetMember(member: string) {
    if ($keyDetail) {
      await deleteZSetMember($keyDetail.key, member);
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

  let copyStatus: Record<string, boolean> = {};

  async function copyText(text: string, id: string) {
    try {
      await navigator.clipboard.writeText(text);
      copyStatus = { ...copyStatus, [id]: true };
      setTimeout(() => {
        copyStatus = { ...copyStatus, [id]: false };
      }, 2000);
    } catch (err) {
      console.error('Failed to copy', err);
    }
  }

  function getValueString(): string {
    if (!$keyDetail) return '';
    if ($keyDetail.value.type === 'String') return $keyDetail.value.data;
    return JSON.stringify($keyDetail.value.data, null, 2);
  }
</script>

<svelte:window on:keydown={handleWindowKeydown} />

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
            <div class="key-title-wrapper">
              <h3 class="key-title mono truncate" title={$keyDetail.key}>{$keyDetail.key}</h3>
              <button class="btn btn-sm btn-icon copy-btn" on:click={() => copyText($keyDetail.key, 'key')} title="Copy Key">
                {#if copyStatus['key']}
                  <Icons name="check" size={13} />
                {:else}
                  <Icons name="copy" size={13} />
                {/if}
              </button>
            </div>
          {/if}
          <div class="key-meta">
            <span class="badge {getTypeBadgeClass($keyDetail.key_type)}">{$keyDetail.key_type}</span>
            <span class="meta-item" title="Size">{$keyDetail.size} {$keyDetail.key_type === 'string' ? 'bytes' : 'items'}</span>
            <span class="meta-item" title="TTL" style="display: inline-flex; align-items: center; gap: 4px;">
              <Icons name="clock" size={12} />
              <span>{formatTtl($keyDetail.ttl)}</span>
            </span>
          </div>
        </div>
        <div class="detail-actions">
          <button class="btn btn-sm" style="display: inline-flex; align-items: center; gap: 4px;" on:click={() => copyText(getValueString(), 'val')} title="Copy Value">
            {#if copyStatus['val']}
              <Icons name="check" size={13} />
              <span>Copied</span>
            {:else}
              <Icons name="copy" size={13} />
              <span>Copy</span>
            {/if}
          </button>
          <button class="btn btn-sm" style="display: inline-flex; align-items: center; gap: 4px;" on:click={startRename} title="Rename key">
            <Icons name="edit" size={13} />
            <span>Rename</span>
          </button>
          <button class="btn btn-sm btn-icon" on:click={() => loadKeyDetail($keyDetail.key)} title="Refresh">
            <Icons name="refresh" size={13} />
          </button>
          <button class="btn btn-sm btn-icon btn-danger" on:click={askDeleteKey} title="Delete key">
            <Icons name="trash" size={13} />
          </button>
        </div>
      </div>

      <!-- TTL Editor -->
      <div class="ttl-row">
        <label class="text-muted" for="ttl-field" style="display: inline-flex; align-items: center; gap: 4px;">
          <Icons name="clock" size={12} />
          <span>TTL (seconds):</span>
        </label>
        <input id="ttl-field" class="input input-mono" style="width: 140px;" bind:value={editTtl} placeholder="No expiry" />
        <button class="btn btn-sm" on:click={saveTtl}>Set TTL</button>
      </div>

      <!-- Value Display -->
      <div class="value-section">
        <!-- Result Search Bar -->
        {#if !editing}
          <div class="result-search-bar animate-fade">
            <div class="result-search-input-wrap">
              <span class="result-search-icon">
                <Icons name="search" size={13} />
              </span>
              <input
                type="text"
                class="input input-sm result-search-input"
                bind:this={resultSearchInputEl}
                bind:value={resultSearchQuery}
                placeholder={getSearchPlaceholder($keyDetail.value.type)}
                on:keydown={handleSearchKeydown}
                spellcheck="false"
              />
              {#if resultSearchQuery}
                <button
                  type="button"
                  class="btn btn-sm btn-icon clear-search-btn"
                  on:click={() => { resultSearchQuery = ''; resultSearchInputEl?.focus(); }}
                  title="Clear search (Esc)"
                >
                  <Icons name="close" size={12} />
                </button>
              {/if}
            </div>

            {#if resultSearchQuery.trim()}
              <div class="search-match-badge animate-fade">
                {#if $keyDetail.value.type === 'Hash'}
                  <span class:no-match={filteredHashData.length === 0}>
                    {filteredHashData.length} / {($keyDetail.value.data as HashField[]).length} fields
                  </span>
                {:else if $keyDetail.value.type === 'List'}
                  <span class:no-match={filteredListData.length === 0}>
                    {filteredListData.length} / {($keyDetail.value.data as string[]).length} items
                  </span>
                {:else if $keyDetail.value.type === 'Set'}
                  <span class:no-match={filteredSetData.length === 0}>
                    {filteredSetData.length} / {($keyDetail.value.data as string[]).length} members
                  </span>
                {:else if $keyDetail.value.type === 'ZSet'}
                  <span class:no-match={filteredZSetData.length === 0}>
                    {filteredZSetData.length} / {($keyDetail.value.data as ZSetMember[]).length} members
                  </span>
                {:else if $keyDetail.value.type === 'String'}
                  <span class:no-match={stringMatchCount === 0}>
                    {stringMatchCount} {stringMatchCount === 1 ? 'match' : 'matches'}
                  </span>
                {/if}
              </div>
            {/if}
          </div>
        {/if}

        {#if $keyDetail.value.type === 'String'}
          <div class="value-header">
            <span class="text-muted">Value</span>
            {#if editing}
              <div class="value-actions">
                <button class="btn btn-sm btn-primary" style="display: inline-flex; align-items: center; gap: 4px;" on:click={saveEdit}>
                  <Icons name="save" size={13} />
                  <span>Save</span>
                </button>
                <button class="btn btn-sm" on:click={cancelEdit}>Cancel</button>
              </div>
            {:else}
              <button class="btn btn-sm" style="display: inline-flex; align-items: center; gap: 4px;" on:click={startEdit}>
                <Icons name="edit" size={13} />
                <span>Edit</span>
              </button>
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
              <pre class="value-display mono json-display">{@html resultSearchQuery.trim() ? highlightInsideHtml(jsonResult.highlighted, resultSearchQuery) : jsonResult.highlighted}</pre>
            {:else}
              <pre class="value-display mono">{@html resultSearchQuery.trim() ? highlightText($keyDetail.value.data, resultSearchQuery) : escapeHtml($keyDetail.value.data)}</pre>
            {/if}
          {:else}
            <pre class="value-display mono">{@html resultSearchQuery.trim() ? highlightText($keyDetail.value.data, resultSearchQuery) : escapeHtml($keyDetail.value.data)}</pre>
          {/if}

        {:else if $keyDetail.value.type === 'Hash'}
          <div class="value-header">
            <span class="text-muted">
              Hash Fields ({filteredHashData.length}{#if resultSearchQuery.trim() && filteredHashData.length !== $keyDetail.size} of {$keyDetail.size}{/if})
            </span>
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
                {#if filteredHashData.length === 0}
                  <tr>
                    <td colspan="3" class="no-matches-cell">
                      <div class="no-matches-content">
                        <span>No fields matching "<strong>{resultSearchQuery}</strong>"</span>
                        <button class="btn btn-sm" on:click={() => resultSearchQuery = ''}>Clear filter</button>
                      </div>
                    </td>
                  </tr>
                {:else}
                  {#each filteredHashData as item}
                    <tr>
                      <td class="mono">
                        {#if resultSearchQuery.trim()}
                          {@html highlightText(item.field, resultSearchQuery)}
                        {:else}
                          {item.field}
                        {/if}
                      </td>
                      <td class="mono">
                        {#if editingHashField === item.field}
                          <input class="input input-mono" bind:value={editingHashValue} on:keydown={(e) => e.key === 'Enter' && saveHashFieldEdit()} />
                        {:else}
                          <div style="display: flex; align-items: center; justify-content: space-between; gap: 4px;">
                            <span class="cell-value" title={item.value}>
                              {#if resultSearchQuery.trim()}
                                {@html highlightText(item.value, resultSearchQuery)}
                              {:else}
                                {item.value}
                              {/if}
                            </span>
                            <button class="btn btn-sm btn-icon copy-btn" on:click={() => copyText(item.value, `hf_${item.field}`)} title="Copy field value">
                              {#if copyStatus[`hf_${item.field}`]}
                                <Icons name="check" size={12} />
                              {:else}
                                <Icons name="copy" size={12} />
                              {/if}
                            </button>
                          </div>
                        {/if}
                      </td>
                      <td>
                        {#if editingHashField === item.field}
                          <button class="btn btn-sm btn-primary" on:click={saveHashFieldEdit}>
                            <Icons name="check" size={12} />
                          </button>
                        {:else}
                          <div style="display: flex; gap: 4px;">
                            <button class="btn btn-sm btn-icon" on:click={() => startEditHashField(item.field, item.value)} title="Edit field">
                              <Icons name="edit" size={12} />
                            </button>
                            <button class="btn btn-sm btn-icon btn-danger" on:click={() => handleDeleteHashField(item.field)} title="Delete field">
                              <Icons name="trash" size={12} />
                            </button>
                          </div>
                        {/if}
                      </td>
                    </tr>
                  {/each}
                {/if}
              </tbody>
            </table>
          </div>
          <div class="add-row">
            <input class="input input-mono" bind:value={newHashField} placeholder="Field name" />
            <input class="input input-mono" bind:value={newHashValue} placeholder="Value" />
            <button class="btn btn-sm btn-primary" style="display: inline-flex; align-items: center; gap: 4px;" on:click={handleAddHashField}>
              <Icons name="plus" size={12} />
              <span>Add</span>
            </button>
          </div>

        {:else if $keyDetail.value.type === 'List'}
          <div class="value-header">
            <span class="text-muted">
              List Items ({filteredListData.length}{#if resultSearchQuery.trim() && filteredListData.length !== $keyDetail.size} of {$keyDetail.size}{/if})
            </span>
          </div>
          {#if filteredListData.length === 0}
            <div class="no-matches-box">
              <span>No items matching "<strong>{resultSearchQuery}</strong>"</span>
              <button class="btn btn-sm" on:click={() => resultSearchQuery = ''}>Clear filter</button>
            </div>
          {:else}
            <div class="list-items">
              {#each filteredListData as { item, originalIndex }}
                <div class="list-item">
                  <span class="item-index">{originalIndex}</span>
                  <span class="mono truncate cell-item-text" title={item}>
                    {#if resultSearchQuery.trim()}
                      {@html highlightText(item, resultSearchQuery)}
                    {:else}
                      {item}
                    {/if}
                  </span>
                  <div class="item-actions">
                    <button class="btn btn-sm btn-icon copy-btn" on:click={() => copyText(item, `li_${originalIndex}`)} title="Copy item value">
                      {#if copyStatus[`li_${originalIndex}`]}
                        <Icons name="check" size={12} />
                      {:else}
                        <Icons name="copy" size={12} />
                      {/if}
                    </button>
                    <button class="btn btn-sm btn-icon btn-danger" on:click={() => handleDeleteListItem(item)} title="Delete item from list">
                      <Icons name="trash" size={12} />
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
          <div class="add-row">
            <input class="input input-mono" bind:value={newListItem} placeholder="New item value" on:keydown={(e) => e.key === 'Enter' && handleAddListItem()} />
            <button class="btn btn-sm btn-primary" style="display: inline-flex; align-items: center; gap: 4px;" on:click={handleAddListItem}>
              <Icons name="plus" size={12} />
              <span>Add</span>
            </button>
          </div>

        {:else if $keyDetail.value.type === 'Set'}
          <div class="value-header">
            <span class="text-muted">
              Set Members ({filteredSetData.length}{#if resultSearchQuery.trim() && filteredSetData.length !== $keyDetail.size} of {$keyDetail.size}{/if})
            </span>
          </div>
          {#if filteredSetData.length === 0}
            <div class="no-matches-box">
              <span>No members matching "<strong>{resultSearchQuery}</strong>"</span>
              <button class="btn btn-sm" on:click={() => resultSearchQuery = ''}>Clear filter</button>
            </div>
          {:else}
            <div class="list-items">
              {#each filteredSetData as { item, originalIndex }}
                <div class="list-item">
                  <span class="mono truncate cell-item-text" title={item}>
                    {#if resultSearchQuery.trim()}
                      {@html highlightText(item, resultSearchQuery)}
                    {:else}
                      {item}
                    {/if}
                  </span>
                  <div class="item-actions">
                    <button class="btn btn-sm btn-icon copy-btn" on:click={() => copyText(item, `set_${originalIndex}`)} title="Copy member value">
                      {#if copyStatus[`set_${originalIndex}`]}
                        <Icons name="check" size={12} />
                      {:else}
                        <Icons name="copy" size={12} />
                      {/if}
                    </button>
                    <button class="btn btn-sm btn-icon btn-danger" on:click={() => handleDeleteSetMember(item)} title="Delete member from set">
                      <Icons name="trash" size={12} />
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
          <div class="add-row">
            <input class="input input-mono" bind:value={newSetMember} placeholder="New member" on:keydown={(e) => e.key === 'Enter' && handleAddSetMember()} />
            <button class="btn btn-sm btn-primary" style="display: inline-flex; align-items: center; gap: 4px;" on:click={handleAddSetMember}>
              <Icons name="plus" size={12} />
              <span>Add</span>
            </button>
          </div>

        {:else if $keyDetail.value.type === 'ZSet'}
          <div class="value-header">
            <span class="text-muted">
              Sorted Set ({filteredZSetData.length}{#if resultSearchQuery.trim() && filteredZSetData.length !== $keyDetail.size} of {$keyDetail.size}{/if})
            </span>
          </div>
          <div class="table-container">
            <table>
              <thead>
                <tr>
                  <th style="width: 120px;">Score</th>
                  <th>Member</th>
                  <th style="width: 80px;">Actions</th>
                </tr>
              </thead>
              <tbody>
                {#if filteredZSetData.length === 0}
                  <tr>
                    <td colspan="3" class="no-matches-cell">
                      <div class="no-matches-content">
                        <span>No members matching "<strong>{resultSearchQuery}</strong>"</span>
                        <button class="btn btn-sm" on:click={() => resultSearchQuery = ''}>Clear filter</button>
                      </div>
                    </td>
                  </tr>
                {:else}
                  {#each filteredZSetData as item, i}
                    <tr>
                      <td class="mono text-accent">
                        {#if resultSearchQuery.trim()}
                          {@html highlightText(String(item.score), resultSearchQuery)}
                        {:else}
                          {item.score}
                        {/if}
                      </td>
                      <td class="mono">
                        {#if resultSearchQuery.trim()}
                          {@html highlightText(item.member, resultSearchQuery)}
                        {:else}
                          {item.member}
                        {/if}
                      </td>
                      <td>
                        <div style="display: flex; gap: 4px;">
                          <button class="btn btn-sm btn-icon copy-btn" on:click={() => copyText(item.member, `zset_${i}`)} title="Copy member">
                            {#if copyStatus[`zset_${i}`]}
                              <Icons name="check" size={12} />
                            {:else}
                              <Icons name="copy" size={12} />
                            {/if}
                          </button>
                          <button class="btn btn-sm btn-icon btn-danger" on:click={() => handleDeleteZSetMember(item.member)} title="Delete member from zset">
                            <Icons name="trash" size={12} />
                          </button>
                        </div>
                      </td>
                    </tr>
                  {/each}
                {/if}
              </tbody>
            </table>
          </div>
          <div class="add-row">
            <input class="input input-mono" style="max-width: 110px;" type="number" step="any" bind:value={newZSetScore} placeholder="Score" />
            <input class="input input-mono" bind:value={newZSetMember} placeholder="New member name" on:keydown={(e) => e.key === 'Enter' && handleAddZSetMember()} />
            <button class="btn btn-sm btn-primary" style="display: inline-flex; align-items: center; gap: 4px;" on:click={handleAddZSetMember}>
              <Icons name="plus" size={12} />
              <span>Add</span>
            </button>
          </div>

        {:else}
          <pre class="value-display mono">{@html resultSearchQuery.trim() ? highlightText($keyDetail.value.data, resultSearchQuery) : escapeHtml($keyDetail.value.data)}</pre>
        {/if}
      </div>
    </div>
  {/if}

  {#if showDeleteConfirm}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="confirm-overlay" on:click={() => showDeleteConfirm = false}>
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
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
  .key-title-wrapper {
    display: flex;
    align-items: center;
    gap: var(--gap-sm);
    margin-bottom: var(--gap-sm);
  }
  .key-title-wrapper .key-title {
    margin-bottom: 0;
  }
  .copy-btn {
    font-size: 14px;
    padding: 2px 6px;
    background: transparent;
    border-color: transparent;
    opacity: 0.5;
    transition: all 0.2s;
  }
  .copy-btn:hover {
    opacity: 1;
    background: var(--bg-hover);
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
  .cell-item-text {
    flex: 1;
    min-width: 0;
  }
  .item-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    opacity: 0.7;
    transition: opacity var(--transition-fast);
  }
  .list-item:hover .item-actions {
    opacity: 1;
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

  /* Result search bar & highlights */
  .result-search-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin-bottom: var(--gap-md);
    padding: 6px 10px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-primary);
    border-radius: var(--radius-sm);
  }
  .result-search-input-wrap {
    position: relative;
    display: flex;
    align-items: center;
    flex: 1;
    min-width: 0;
  }
  .result-search-icon {
    position: absolute;
    left: 8px;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    pointer-events: none;
  }
  .result-search-input {
    width: 100%;
    height: 28px;
    padding-left: 28px;
    padding-right: 28px;
    font-size: 11px;
    background: var(--bg-primary);
    border: 1px solid var(--border-secondary);
    border-radius: var(--radius-sm);
    color: var(--text-primary);
  }
  .result-search-input:focus {
    border-color: var(--accent);
    outline: none;
  }
  .clear-search-btn {
    position: absolute;
    right: 4px;
    padding: 2px;
    height: 20px;
    width: 20px;
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 2px;
  }
  .clear-search-btn:hover {
    color: var(--text-primary);
    background: var(--bg-hover);
  }
  .search-match-badge {
    font-size: 11px;
    padding: 2px 8px;
    border-radius: 10px;
    background: rgba(0, 212, 255, 0.12);
    color: #00d4ff;
    border: 1px solid rgba(0, 212, 255, 0.25);
    white-space: nowrap;
    font-weight: 500;
    flex-shrink: 0;
  }
  .search-match-badge .no-match {
    color: #f87171;
    background: rgba(248, 113, 113, 0.12);
    border-color: rgba(248, 113, 113, 0.25);
  }
  :global(.search-match) {
    background: rgba(255, 204, 0, 0.35);
    color: #fff;
    padding: 1px 3px;
    border-radius: 2px;
    font-weight: 600;
    text-shadow: 0 0 2px rgba(0,0,0,0.8);
    border-bottom: 2px solid #ffcc00;
  }
  .no-matches-cell {
    padding: var(--gap-lg) !important;
    text-align: center;
  }
  .no-matches-content, .no-matches-box {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: var(--gap-lg);
    color: var(--text-muted);
    font-size: 12px;
    background: var(--bg-primary);
    border-radius: var(--radius-sm);
  }
</style>
