<script lang="ts">
  import {
    connectRedis,
    testConnection,
    savedConnections,
    saveConnection,
    removeConnection,
    error,
    isLoading,
  } from '$lib/stores/redis';
  import type { SavedConnection } from '$lib/types';

  export let onConnected: () => void = () => {};

  let url = '127.0.0.1:6379';
  let password = '';
  let connectionName = '';
  let testing = false;
  let testResult: boolean | null = null;
  let connecting = false;
  let showSaved = true;

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
      await connectRedis(url, password || undefined, connectionName.trim() || undefined);
      // Auto-save connection
      if (connectionName.trim()) {
        saveConnection({
          id: generateId(),
          name: connectionName.trim(),
          url,
          password,
        });
      }
      onConnected();
    } catch (e) {
      // error is set in store
    }
    connecting = false;
  }

  function loadFromSaved(conn: SavedConnection) {
    url = conn.url;
    password = conn.password;
    connectionName = conn.name;
  }

  function deleteSaved(id: string) {
    removeConnection(id);
  }

  $: isCluster = url.includes(',');
</script>

<div class="connection-page">
  <div class="connection-container animate-fade">
    <div class="logo-section">
      <div class="logo">
        <svg width="48" height="48" viewBox="0 0 24 24" fill="none">
          <path d="M12 2L2 7v10l10 5 10-5V7L12 2z" stroke="var(--accent)" stroke-width="1.5" fill="rgba(0,212,255,0.1)"/>
          <path d="M12 22V12M2 7l10 5 10-5" stroke="var(--accent)" stroke-width="1.5"/>
          <circle cx="12" cy="12" r="2" fill="var(--accent)"/>
        </svg>
      </div>
      <h1>Redis Manager</h1>
      <p class="text-muted">Lightweight Redis Management Tool</p>
    </div>

    <div class="form-section">
      <div class="form-grid">
        <div class="field full">
          <label>
            Connection Name
            <span class="hint-inline">(VD: DEV, UAT, PROD)</span>
          </label>
          <input
            class="input"
            bind:value={connectionName}
            placeholder="My Redis DEV"
          />
        </div>

        <div class="field full">
          <label>
            Host(s)
            {#if isCluster}
              <span class="badge badge-hash" style="margin-left: 8px;">CLUSTER</span>
            {:else}
              <span class="badge badge-string" style="margin-left: 8px;">STANDALONE</span>
            {/if}
          </label>
          <input
            class="input input-mono"
            bind:value={url}
            placeholder="127.0.0.1:6379 or host1:6379,host2:6380"
          />
          <span class="hint">Nhiều host ngăn bởi dấu , để connect Cluster</span>
        </div>

        <div class="field full">
          <label>Password</label>
          <input
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
        <button class="btn" on:click={handleTest} disabled={testing || !url}>
          {#if testing}
            <span class="animate-spin">⟳</span> Testing...
          {:else}
            ⚡ Test
          {/if}
        </button>
        <button class="btn btn-primary" on:click={handleConnect} disabled={connecting || !url}>
          {#if connecting}
            <span class="animate-spin">⟳</span> Connecting...
          {:else}
            → Connect
          {/if}
        </button>
      </div>
    </div>

    {#if $savedConnections.length > 0}
      <div class="saved-section">
        <button class="saved-toggle" on:click={() => showSaved = !showSaved}>
          <span class="toggle-icon" class:open={showSaved}>▸</span>
          Saved Connections ({$savedConnections.length})
        </button>
        {#if showSaved}
          <div class="saved-list animate-fade">
            {#each $savedConnections as conn}
              <div class="saved-item" on:click={() => loadFromSaved(conn)}>
                <div class="saved-info">
                  <span class="saved-name">{conn.name}</span>
                  <span class="saved-url mono">{conn.url}</span>
                </div>
                <button
                  class="btn btn-icon btn-sm btn-danger"
                  on:click|stopPropagation={() => deleteSaved(conn.id)}
                  title="Delete"
                >✕</button>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </div>
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
    padding: 16px;
    background: rgba(0, 212, 255, 0.06);
    border-radius: var(--radius-lg);
    margin-bottom: var(--gap-md);
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
  .toggle-icon { transition: transform var(--transition-fast); display: inline-block; }
  .toggle-icon.open { transform: rotate(90deg); }
  .saved-list { margin-top: var(--gap-md); display: flex; flex-direction: column; gap: var(--gap-sm); }
  .saved-item {
    display: flex; align-items: center; justify-content: space-between;
    padding: var(--gap-sm) var(--gap-md);
    background: var(--bg-tertiary); border: 1px solid var(--border-primary);
    border-radius: var(--radius-sm); cursor: pointer;
    transition: all var(--transition-fast);
  }
  .saved-item:hover { background: var(--bg-hover); border-color: var(--border-accent); }
  .saved-info { display: flex; flex-direction: column; gap: 2px; }
  .saved-name { font-weight: 500; font-size: 13px; }
  .saved-url { font-size: 11px; color: var(--text-muted); }
</style>
