<script lang="ts">
  import {
    connectRedis,
    testConnection,
    savedConnections,
    saveConnection,
    removeConnection,
    error,
    isLoading,
    isTauriEnvironment,
  } from '$lib/stores/redis';
  import type { SavedConnection } from '$lib/types';
  import Icons from './Icons.svelte';
  import AboutModal from './AboutModal.svelte';

  export let onConnected: () => void = () => {};

  let isTauri = isTauriEnvironment();
  let url = '127.0.0.1:6379';
  let password = '';
  let connectionName = '';
  let testing = false;
  let testResult: boolean | null = null;
  let connecting = false;
  let showSaved = true;
  let showAboutModal = false;

  function generateId() {
    return Date.now().toString(36) + Math.random().toString(36).substr(2);
  }

  async function handleTest() {
    testing = true;
    testResult = null;
    testResult = await testConnection(url, password || undefined);
    testing = false;
  }

  async function handleConnect() {
    connecting = true;
    try {
      const connName = connectionName.trim() || url.split(',')[0] || 'Redis';
      await connectRedis(url, password || undefined, connName);
      // Always save connection
      saveConnection({
        id: generateId(),
        name: connName,
        url,
        password,
      });
      onConnected();
    } catch (e) {
      // error is set in store
    }
    connecting = false;
  }

  let connectingId: string | null = null;
  let confirmDeleteId: string | null = null;
  let confirmDeleteName: string = '';

  async function loadFromSaved(conn: SavedConnection) {
    url = conn.url;
    password = conn.password;
    connectionName = conn.name;
    connectingId = conn.id;
    connecting = true;
    try {
      await connectRedis(conn.url, conn.password || undefined, conn.name);
      saveConnection(conn);
      onConnected();
    } catch (e) {
      // error is set in store, user can retry manually
    }
    connecting = false;
    connectingId = null;
  }

  function askDeleteSaved(id: string, name: string) {
    confirmDeleteId = id;
    confirmDeleteName = name;
  }

  function doDeleteSaved() {
    if (confirmDeleteId) removeConnection(confirmDeleteId);
    confirmDeleteId = null;
  }

  $: isCluster = url.includes(',');
</script>

<div class="connection-page">
  <div class="connection-container animate-fade">
    <div class="logo-section">
      <div class="logo">
        <img src="/favicon.png" alt="Redis Manager Logo" class="app-logo-large" />
      </div>
      <h1>Redis Manager</h1>
      <p class="text-muted">Lightweight, ultra-fast Redis desktop GUI</p>
    </div>

    <div class="form-section">
      {#if !isTauri}
        <div class="web-mode-banner animate-fade">
          <div class="banner-badge">
            <Icons name="info" size={15} />
            <span>CHẾ ĐỘ WEB PREVIEW</span>
          </div>
          <p class="banner-text">
            Bạn đang mở ứng dụng trên trình duyệt web thông qua <code>yarn dev</code>. Trình duyệt không thể kết nối trực tiếp đến Redis TCP socket nếu thiếu backend Rust.
          </p>
          <div class="banner-guide">
            <span class="guide-title">Khởi chạy ứng dụng Desktop để kết nối Redis:</span>
            <div class="cmd-row">
              <code>yarn tauri dev</code>
              <span class="cmd-alt">(hoặc <code>yarn dev:app</code>)</span>
            </div>
          </div>
        </div>
      {/if}

      <div class="form-grid">
        <div class="field full">
          <label for="conn-name">
            Connection Name
            <span class="hint-inline">(VD: DEV, UAT, PROD)</span>
          </label>
          <input
            id="conn-name"
            class="input"
            bind:value={connectionName}
            placeholder="My Redis DEV"
          />
        </div>

        <div class="field full">
          <label for="conn-url">
            Host(s)
            {#if isCluster}
              <span class="badge badge-hash" style="margin-left: 8px;">CLUSTER</span>
            {:else}
              <span class="badge badge-string" style="margin-left: 8px;">STANDALONE</span>
            {/if}
          </label>
          <input
            id="conn-url"
            class="input input-mono"
            bind:value={url}
            placeholder="127.0.0.1:6379 or host1:6379,host2:6380"
          />
          <span class="hint">Nhiều host ngăn bởi dấu , để connect Cluster</span>
        </div>

        <div class="field full">
          <label for="conn-password">Password</label>
          <input
            id="conn-password"
            class="input"
            type="password"
            bind:value={password}
            placeholder="Optional"
          />
        </div>
      </div>

      {#if $error}
        <div class="error-msg animate-fade">
          <span>⚠</span> {$error}
        </div>
      {/if}

      {#if testResult !== null}
        <div class="test-result animate-fade" class:success={testResult} class:fail={!testResult}>
          {testResult ? '✓ Connection successful!' : '✗ Connection failed'}
        </div>
      {/if}

      <div class="actions">
        <button class="btn" on:click={handleTest} disabled={testing || !url} style="display: inline-flex; align-items: center; justify-content: center; gap: 6px;">
          {#if testing}
            <span class="animate-spin" style="display: flex;"><Icons name="refresh" size={13} /></span>
            <span>Testing...</span>
          {:else}
            <Icons name="bolt" size={13} />
            <span>Test</span>
          {/if}
        </button>
        <button class="btn btn-primary" on:click={handleConnect} disabled={connecting || !url} style="display: inline-flex; align-items: center; justify-content: center; gap: 6px;">
          {#if connecting}
            <span class="animate-spin" style="display: flex;"><Icons name="refresh" size={13} /></span>
            <span>Connecting...</span>
          {:else}
            <Icons name="play" size={13} />
            <span>Connect</span>
          {/if}
        </button>
      </div>
    </div>

    {#if $savedConnections.length > 0}
      <div class="saved-section">
        <button class="saved-toggle" on:click={() => showSaved = !showSaved}>
          <span class="toggle-icon">
            <Icons name={showSaved ? 'chevron-down' : 'chevron-right'} size={12} />
          </span>
          <span>Saved Connections ({$savedConnections.length})</span>
        </button>
        {#if showSaved}
          <div class="saved-list animate-fade">
            {#each $savedConnections as conn (conn.id)}
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div class="saved-item" class:connecting={connectingId === conn.id} on:click={() => !connecting && loadFromSaved(conn)}>
                <div class="saved-info">
                  <span class="saved-name">
                    {#if connectingId === conn.id}
                      <span class="animate-spin" style="display: inline-flex; margin-right: 4px;"><Icons name="refresh" size={11} /></span>
                    {/if}
                    {conn.name}
                  </span>
                  <span class="saved-url mono">{conn.url}</span>
                </div>
                <button
                  class="btn btn-icon btn-sm btn-danger"
                  on:click|stopPropagation={() => askDeleteSaved(conn.id, conn.name)}
                  title="Delete"
                >
                  <Icons name="trash" size={12} />
                </button>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/if}

    <div class="form-footer">
      <button type="button" class="btn-author-link" on:click={() => showAboutModal = true}>
        <Icons name="info" size={12} />
        <span>Tác giả: Ngô Minh Đức (@minhduc2001)</span>
      </button>
    </div>

    {#if confirmDeleteId}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="confirm-overlay" on:click={() => confirmDeleteId = null}>
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="confirm-modal animate-fade" on:click|stopPropagation>
          <p>Delete saved connection <strong>{confirmDeleteName}</strong>?</p>
          <div class="confirm-actions">
            <button class="btn btn-sm" on:click={() => confirmDeleteId = null}>Cancel</button>
            <button class="btn btn-sm btn-danger" on:click={doDeleteSaved}>Delete</button>
          </div>
        </div>
      </div>
    {/if}
  </div>

  {#if showAboutModal}
    <AboutModal onClose={() => showAboutModal = false} />
  {/if}
</div>

<style>
  .connection-page {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
    background:
      radial-gradient(ellipse at 30% 20%, rgba(0, 212, 255, 0.05) 0%, transparent 50%),
      radial-gradient(ellipse at 70% 80%, rgba(171, 71, 188, 0.05) 0%, transparent 50%),
      var(--bg-primary);
  }
  .connection-container {
    width: 480px;
    max-height: 90vh;
    overflow-y: auto;
    padding: var(--gap-2xl);
    background: var(--bg-card);
    backdrop-filter: blur(20px);
    border: 1px solid var(--border-primary);
    border-radius: var(--radius-xl);
    box-shadow: var(--shadow-lg);
  }
  .logo-section { text-align: center; margin-bottom: var(--gap-xl); }
  .logo {
    display: inline-flex;
    padding: 10px;
    background: rgba(0, 212, 255, 0.06);
    border-radius: var(--radius-xl);
    margin-bottom: var(--gap-md);
    box-shadow: 0 4px 20px rgba(0, 212, 255, 0.15);
  }
  .app-logo-large {
    width: 64px;
    height: 64px;
    border-radius: 14px;
    object-fit: cover;
  }
  h1 { font-size: 24px; font-weight: 700; margin-bottom: var(--gap-xs); }

  .form-grid {
    display: flex;
    flex-direction: column;
    gap: var(--gap-md);
  }
  .field { display: flex; flex-direction: column; gap: var(--gap-xs); }
  .field.full { width: 100%; }
  label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    gap: var(--gap-sm);
  }
  .hint { font-size: 11px; color: var(--text-muted); }
  .hint-inline { font-weight: 400; color: var(--text-muted); }

  .error-msg {
    margin-top: var(--gap-md);
    padding: var(--gap-sm) var(--gap-md);
    background: rgba(255, 82, 82, 0.1);
    border: 1px solid rgba(255, 82, 82, 0.2);
    border-radius: var(--radius-sm);
    color: var(--error);
    font-size: 12px;
  }
  .test-result {
    margin-top: var(--gap-md);
    padding: var(--gap-sm) var(--gap-md);
    border-radius: var(--radius-sm);
    font-size: 12px;
    font-weight: 500;
  }
  .test-result.success {
    background: rgba(0, 230, 118, 0.1);
    border: 1px solid rgba(0, 230, 118, 0.2);
    color: var(--success);
  }
  .test-result.fail {
    background: rgba(255, 82, 82, 0.1);
    border: 1px solid rgba(255, 82, 82, 0.2);
    color: var(--error);
  }
  .actions {
    display: flex;
    gap: var(--gap-md);
    margin-top: var(--gap-xl);
  }
  .actions .btn { flex: 1; justify-content: center; padding: 10px 16px; }

  .saved-section {
    margin-top: var(--gap-xl);
    border-top: 1px solid var(--border-primary);
    padding-top: var(--gap-lg);
  }
  .saved-toggle {
    background: none; border: none; color: var(--text-secondary);
    font-size: 12px; font-weight: 500; cursor: pointer;
    display: flex; align-items: center; gap: var(--gap-sm);
    font-family: var(--font-sans);
  }
  .saved-toggle:hover { color: var(--text-primary); }
  .toggle-icon { display: inline-flex; align-items: center; }
  .saved-list { margin-top: var(--gap-md); display: flex; flex-direction: column; gap: var(--gap-sm); }
  .saved-item {
    display: flex; align-items: center; gap: var(--gap-sm);
    padding: var(--gap-sm) var(--gap-md);
    background: var(--bg-tertiary); border: 1px solid var(--border-primary);
    border-radius: var(--radius-sm); cursor: pointer;
    transition: all var(--transition-fast);
  }
  .saved-item:hover { background: var(--bg-hover); border-color: var(--border-accent); }
  .saved-info { display: flex; flex-direction: column; gap: 2px; flex: 1; min-width: 0; overflow: hidden; }
  .saved-name { font-weight: 500; font-size: 13px; display: flex; align-items: center; gap: var(--gap-xs); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .saved-url { font-size: 11px; color: var(--text-muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .saved-item.connecting { border-color: var(--accent); opacity: 0.8; pointer-events: none; }

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
    min-width: 300px;
    box-shadow: var(--shadow-lg);
  }
  .confirm-modal p { font-size: 13px; margin-bottom: var(--gap-md); }
  .confirm-actions { display: flex; gap: var(--gap-sm); justify-content: flex-end; }

  .form-footer {
    margin-top: var(--gap-lg);
    display: flex;
    justify-content: center;
  }
  .btn-author-link {
    background: none;
    border: none;
    color: var(--text-muted);
    font-size: 11px;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border-radius: var(--radius-sm);
    transition: all var(--transition-fast);
  }
  .btn-author-link:hover {
    color: var(--accent);
    background: rgba(0, 212, 255, 0.08);
  }

  .web-mode-banner {
    background: rgba(245, 158, 11, 0.07);
    border: 1px solid rgba(245, 158, 11, 0.28);
    border-radius: var(--radius-md);
    padding: 14px 16px;
    margin-bottom: var(--gap-lg);
    display: flex;
    flex-direction: column;
    gap: 10px;
    text-align: left;
  }
  .banner-badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    font-weight: 700;
    color: #f59e0b;
    letter-spacing: 0.5px;
    text-transform: uppercase;
  }
  .banner-text {
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.5;
    margin: 0;
  }
  .banner-text code {
    background: rgba(255, 255, 255, 0.08);
    padding: 2px 5px;
    border-radius: 4px;
    color: #f59e0b;
    font-family: monospace;
    font-size: 11px;
  }
  .banner-guide {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .guide-title {
    font-size: 11px;
    color: var(--text-muted);
  }
  .cmd-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .cmd-row code {
    background: #0b1120;
    border: 1px solid rgba(0, 212, 255, 0.35);
    color: #00d4ff;
    padding: 6px 12px;
    border-radius: var(--radius-sm);
    font-family: monospace;
    font-size: 12px;
    font-weight: 600;
    user-select: all;
  }
  .cmd-alt {
    font-size: 11px;
    color: var(--text-muted);
  }
  .cmd-alt code {
    background: transparent;
    border: none;
    padding: 0;
    color: var(--text-secondary);
    font-size: 11px;
  }
</style>
