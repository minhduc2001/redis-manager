<script lang="ts">
  import { createKey } from '$lib/stores/redis';
  import Icons from './Icons.svelte';

  export let onClose: () => void = () => {};

  let keyName = '';
  let keyType: 'string' | 'hash' | 'list' | 'set' | 'zset' = 'string';
  let stringValue = '';
  let hashField = '';
  let hashValue = '';
  let listItem = '';
  let setMember = '';
  let zsetScore = '0';
  let zsetMember = '';
  let ttl = '';
  let isSubmitting = false;
  let formError = '';

  const types = [
    { id: 'string', label: 'String', badge: 'badge-string' },
    { id: 'hash', label: 'Hash', badge: 'badge-hash' },
    { id: 'list', label: 'List', badge: 'badge-list' },
    { id: 'set', label: 'Set', badge: 'badge-set' },
    { id: 'zset', label: 'ZSet', badge: 'badge-zset' },
  ] as const;

  function setTtlPreset(seconds: number) {
    ttl = seconds > 0 ? String(seconds) : '';
  }

  async function handleSubmit() {
    formError = '';
    const trimmedKey = keyName.trim();
    if (!trimmedKey) {
      formError = 'Key name is required';
      return;
    }

    let payload = '';
    if (keyType === 'string') {
      payload = stringValue;
    } else if (keyType === 'hash') {
      if (!hashField.trim()) {
        formError = 'Hash field name is required';
        return;
      }
      payload = `${hashField.trim()}:${hashValue}`;
    } else if (keyType === 'list') {
      if (!listItem.trim()) {
        formError = 'List item value is required';
        return;
      }
      payload = listItem;
    } else if (keyType === 'set') {
      if (!setMember.trim()) {
        formError = 'Set member value is required';
        return;
      }
      payload = setMember;
    } else if (keyType === 'zset') {
      if (!zsetMember.trim()) {
        formError = 'ZSet member is required';
        return;
      }
      payload = `${zsetScore.trim() || '0'} ${zsetMember.trim()}`;
    }

    const ttlNum = ttl.trim() ? parseInt(ttl.trim()) : undefined;
    if (ttlNum !== undefined && (isNaN(ttlNum) || ttlNum < 0)) {
      formError = 'TTL must be a valid positive number';
      return;
    }

    isSubmitting = true;
    try {
      await createKey(trimmedKey, keyType, payload, ttlNum);
      onClose();
    } catch (e: any) {
      formError = e.toString();
    } finally {
      isSubmitting = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onClose();
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) handleSubmit();
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="modal-overlay" on:click={onClose}>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-box animate-fade" on:click|stopPropagation>
    <div class="modal-header">
      <div class="modal-title-wrap">
        <span class="icon-sparkle" style="color: var(--accent); display: flex;">
          <Icons name="sparkle" size={16} />
        </span>
        <h3>Create New Key</h3>
      </div>
      <button class="btn-close" on:click={onClose} title="Close (Esc)">✕</button>
    </div>

    <form on:submit|preventDefault={handleSubmit} class="modal-body">
      <!-- Key Name -->
      <div class="form-group">
        <label for="key-name">Key Name <span class="required">*</span></label>
        <!-- svelte-ignore a11y_autofocus -->
        <input
          id="key-name"
          class="input input-mono"
          bind:value={keyName}
          placeholder="e.g. user:1001:profile or app:config"
          autofocus
        />
      </div>

      <!-- Key Type -->
      <div class="form-group">
        <span class="label-text">Data Type</span>
        <div class="type-selector">
          {#each types as t}
            <button
              type="button"
              class="type-pill"
              class:selected={keyType === t.id}
              on:click={() => keyType = t.id}
            >
              <span class="badge {t.badge}">{t.label}</span>
            </button>
          {/each}
        </div>
      </div>

      <!-- Value Input Based on Type -->
      {#if keyType === 'string'}
        <div class="form-group">
          <label for="string-value">String Value</label>
          <textarea
            id="string-value"
            class="input input-mono textarea"
            rows="5"
            bind:value={stringValue}
            placeholder="Enter string value or paste JSON..."
          ></textarea>
        </div>
      {:else if keyType === 'hash'}
        <div class="form-row">
          <div class="form-group flex-1">
            <label for="hash-field">Field Name <span class="required">*</span></label>
            <input id="hash-field" class="input input-mono" bind:value={hashField} placeholder="e.g. name or email" />
          </div>
          <div class="form-group flex-2">
            <label for="hash-val">Field Value</label>
            <input id="hash-val" class="input input-mono" bind:value={hashValue} placeholder="Value for field" />
          </div>
        </div>
      {:else if keyType === 'list'}
        <div class="form-group">
          <label for="list-item">Initial Item Value <span class="required">*</span></label>
          <input id="list-item" class="input input-mono" bind:value={listItem} placeholder="First element in list" />
        </div>
      {:else if keyType === 'set'}
        <div class="form-group">
          <label for="set-member">Initial Member Value <span class="required">*</span></label>
          <input id="set-member" class="input input-mono" bind:value={setMember} placeholder="Member value" />
        </div>
      {:else if keyType === 'zset'}
        <div class="form-row">
          <div class="form-group flex-1">
            <label for="zset-score">Score</label>
            <input id="zset-score" class="input input-mono" type="number" step="any" bind:value={zsetScore} placeholder="0" />
          </div>
          <div class="form-group flex-2">
            <label for="zset-member">Member <span class="required">*</span></label>
            <input id="zset-member" class="input input-mono" bind:value={zsetMember} placeholder="Member name" />
          </div>
        </div>
      {/if}

      <!-- TTL (Expiration) -->
      <div class="form-group">
        <div class="label-with-presets">
          <label for="ttl-input">TTL (Seconds, optional)</label>
          <div class="ttl-presets">
            <button type="button" class="preset-chip" on:click={() => setTtlPreset(-1)}>No Expiry</button>
            <button type="button" class="preset-chip" on:click={() => setTtlPreset(60)}>1m</button>
            <button type="button" class="preset-chip" on:click={() => setTtlPreset(3600)}>1h</button>
            <button type="button" class="preset-chip" on:click={() => setTtlPreset(86400)}>1d</button>
          </div>
        </div>
        <input
          id="ttl-input"
          class="input input-mono"
          type="number"
          min="1"
          bind:value={ttl}
          placeholder="Leave blank for no expiration"
        />
      </div>

      {#if formError}
        <div class="error-banner animate-fade">
          <span>⚠</span> {formError}
        </div>
      {/if}

      <div class="modal-footer">
        <button type="button" class="btn" on:click={onClose}>Cancel</button>
        <button type="submit" class="btn btn-primary" disabled={isSubmitting || !keyName.trim()} style="display: inline-flex; align-items: center; gap: 6px;">
          {#if isSubmitting}
            <span class="animate-spin" style="display: flex;"><Icons name="refresh" size={13} /></span>
            <span>Creating...</span>
          {:else}
            <Icons name="plus" size={13} />
            <span>Create Key</span>
          {/if}
        </button>
      </div>
    </form>
  </div>
</div>

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.75);
    backdrop-filter: blur(6px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }
  .modal-box {
    background: var(--bg-secondary);
    border: 1px solid var(--border-secondary);
    border-radius: var(--radius-xl);
    width: 90%;
    max-width: 520px;
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.6), 0 0 24px rgba(0, 212, 255, 0.08);
    overflow: hidden;
  }
  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--gap-lg) var(--gap-xl);
    border-bottom: 1px solid var(--border-primary);
  }
  .modal-title-wrap {
    display: flex;
    align-items: center;
    gap: var(--gap-sm);
  }
  .modal-title-wrap h3 {
    font-size: 15px;
    font-weight: 600;
  }
  .icon-sparkle {
    font-size: 16px;
  }
  .btn-close {
    background: none;
    border: none;
    color: var(--text-muted);
    font-size: 14px;
    cursor: pointer;
    padding: 4px 8px;
    border-radius: var(--radius-sm);
    transition: all var(--transition-fast);
  }
  .btn-close:hover {
    color: var(--text-primary);
    background: var(--bg-hover);
  }

  .modal-body {
    padding: var(--gap-xl);
    display: flex;
    flex-direction: column;
    gap: var(--gap-md);
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .form-group label, .label-text {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  .required {
    color: var(--error);
  }
  .form-row {
    display: flex;
    gap: var(--gap-md);
  }
  .flex-1 { flex: 1; }
  .flex-2 { flex: 2; }

  .textarea {
    resize: vertical;
    min-height: 90px;
    line-height: 1.4;
  }

  .type-selector {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .type-pill {
    background: var(--bg-tertiary);
    border: 1px solid var(--border-primary);
    border-radius: var(--radius-sm);
    padding: 4px 8px;
    cursor: pointer;
    transition: all var(--transition-fast);
  }
  .type-pill:hover {
    border-color: var(--border-accent);
  }
  .type-pill.selected {
    background: var(--bg-active);
    border-color: var(--accent);
    box-shadow: 0 0 8px var(--accent-glow);
  }

  .label-with-presets {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .ttl-presets {
    display: flex;
    gap: 4px;
  }
  .preset-chip {
    background: var(--bg-tertiary);
    border: 1px solid var(--border-primary);
    color: var(--text-muted);
    font-size: 10px;
    padding: 1px 6px;
    border-radius: 4px;
    cursor: pointer;
    transition: all var(--transition-fast);
  }
  .preset-chip:hover {
    color: var(--accent);
    border-color: var(--accent);
  }

  .error-banner {
    padding: var(--gap-sm) var(--gap-md);
    background: rgba(255, 82, 82, 0.1);
    border: 1px solid rgba(255, 82, 82, 0.25);
    border-radius: var(--radius-sm);
    color: var(--error);
    font-size: 12px;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: var(--gap-md);
    margin-top: var(--gap-md);
  }
</style>
