<script lang="ts">
  import {
    isConnected,
    connectionTabs,
    activeConnectionId,
    serverInfo,
    disconnectRedis,
    switchConnection,
    error,
  } from '$lib/stores/redis';

  export let onAddConnection: () => void = () => {};

  let confirmDisconnectId: string | null = null;
  let confirmDisconnectName: string = '';

  $: uptime = $serverInfo ? formatUptime(parseInt($serverInfo.uptime_in_seconds)) : '';

  function formatUptime(seconds: number): string {
    if (isNaN(seconds)) return 'N/A';
    const d = Math.floor(seconds / 86400);
    const h = Math.floor((seconds % 86400) / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    if (d > 0) return `${d}d ${h}h`;
    if (h > 0) return `${h}h ${m}m`;
    return `${m}m`;
  }

  async function handleSwitch(id: string) {
    if (id !== $activeConnectionId) {
      await switchConnection(id);
    }
  }

  function askDisconnect(id: string, name: string) {
    confirmDisconnectId = id;
    confirmDisconnectName = name;
  }

  async function doDisconnect() {
    if (confirmDisconnectId) {
      await disconnectRedis(confirmDisconnectId);
    }
    confirmDisconnectId = null;
  }
</script>

<div class="sidebar">
  <div class="sidebar-header">
    <div class="app-logo">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none">
        <path d="M12 2L2 7v10l10 5 10-5V7L12 2z" stroke="var(--accent)" stroke-width="1.5" fill="rgba(0,212,255,0.1)"/>
        <path d="M12 22V12M2 7l10 5 10-5" stroke="var(--accent)" stroke-width="1.5"/>
        <circle cx="12" cy="12" r="2" fill="var(--accent)"/>
      </svg>
      <span class="app-title">Redis Manager</span>
    </div>
  </div>

  <!-- Connection Tabs -->
  {#if $connectionTabs.length > 0}
    <div class="conn-tabs">
      <div class="tabs-header">
        <span class="tabs-label">Connections</span>
        <button class="btn btn-sm btn-icon" on:click={onAddConnection} title="Add connection">+</button>
      </div>
      {#each $connectionTabs as tab (tab.id)}
        <div
          class="conn-tab"
          class:active={tab.is_active}
          on:click={() => handleSwitch(tab.id)}
        >
          <div class="tab-dot" class:active={tab.is_active}></div>
          <div class="tab-info">
            <span class="tab-name">{tab.name}</span>
            <span class="tab-mode badge {tab.mode === 'cluster' ? 'badge-hash' : 'badge-string'}">{tab.mode}</span>
          </div>
          <button
            class="tab-close"
            on:click|stopPropagation={() => askDisconnect(tab.id, tab.name)}
            title="Disconnect"
          >✕</button>
        </div>
      {/each}
    </div>
  {/if}

  {#if $isConnected && $serverInfo}
    <div class="server-info animate-fade">
      <div class="info-grid">
        <div class="info-item">
          <span class="info-label">Version</span>
          <span class="info-value mono">{$serverInfo.version}</span>
        </div>
        <div class="info-item">
          <span class="info-label">Memory</span>
          <span class="info-value mono">{$serverInfo.used_memory_human}</span>
        </div>
        <div class="info-item">
          <span class="info-label">Keys</span>
          <span class="info-value mono">{$serverInfo.total_keys}</span>
        </div>
        <div class="info-item">
          <span class="info-label">Clients</span>
          <span class="info-value mono">{$serverInfo.connected_clients}</span>
        </div>
        <div class="info-item">
          <span class="info-label">Uptime</span>
          <span class="info-value mono">{uptime}</span>
        </div>
      </div>
    </div>
  {/if}

  {#if $error}
    <div class="error-banner animate-fade">
      <span>⚠ {$error}</span>
      <button class="btn btn-sm btn-icon" on:click={() => error.set(null)}>✕</button>
    </div>
  {/if}

  {#if confirmDisconnectId}
    <div class="confirm-overlay" on:click={() => confirmDisconnectId = null}>
      <div class="confirm-modal animate-fade" on:click|stopPropagation>
        <p>Disconnect <strong>{confirmDisconnectName}</strong>?</p>
        <div class="confirm-actions">
          <button class="btn btn-sm" on:click={() => confirmDisconnectId = null}>Cancel</button>
          <button class="btn btn-sm btn-danger" on:click={doDisconnect}>Disconnect</button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-secondary);
    border-right: 1px solid var(--border-primary);
    overflow-y: auto;
  }
  .sidebar-header {
    padding: var(--gap-md) var(--gap-lg);
    border-bottom: 1px solid var(--border-primary);
  }
  .app-logo { display: flex; align-items: center; gap: var(--gap-sm); }
  .app-title {
    font-size: 14px; font-weight: 700;
    background: linear-gradient(135deg, var(--accent), #a78bfa);
    -webkit-background-clip: text; -webkit-text-fill-color: transparent;
    background-clip: text;
  }

  /* Connection tabs */
  .conn-tabs {
    border-bottom: 1px solid var(--border-primary);
    padding: var(--gap-sm) 0;
  }
  .tabs-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 0 var(--gap-md) var(--gap-xs);
    font-size: 10px; font-weight: 600;
    text-transform: uppercase; letter-spacing: 0.5px;
    color: var(--text-muted);
  }
  .conn-tab {
    display: flex; align-items: center; gap: var(--gap-sm);
    padding: 6px var(--gap-md);
    cursor: pointer; transition: all var(--transition-fast);
    border-left: 2px solid transparent;
  }
  .conn-tab:hover { background: var(--bg-hover); }
  .conn-tab.active {
    background: var(--bg-active);
    border-left-color: var(--accent);
  }
  .tab-dot {
    width: 8px; height: 8px; border-radius: 50%;
    background: var(--text-muted); flex-shrink: 0;
  }
  .tab-dot.active {
    background: var(--success);
    box-shadow: 0 0 6px var(--success);
  }
  .tab-info { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 1px; }
  .tab-name { font-size: 12px; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .tab-mode { width: fit-content; font-size: 9px; }
  .tab-close {
    background: none; border: none; color: var(--text-muted);
    cursor: pointer; padding: 2px 4px; font-size: 10px;
    border-radius: 3px; opacity: 0; transition: all var(--transition-fast);
  }
  .conn-tab:hover .tab-close { opacity: 1; }
  .tab-close:hover { color: var(--error); background: rgba(255,82,82,0.1); }

  /* Server info */
  .server-info {
    padding: var(--gap-md) var(--gap-lg);
    border-bottom: 1px solid var(--border-primary);
  }
  .info-grid {
    display: grid; grid-template-columns: 1fr 1fr;
    gap: var(--gap-xs) var(--gap-md);
  }
  .info-item { display: flex; flex-direction: column; gap: 1px; }
  .info-label {
    font-size: 10px; color: var(--text-muted);
    text-transform: uppercase; letter-spacing: 0.5px;
  }
  .info-value { font-size: 12px; }

  .error-banner {
    margin: var(--gap-sm) var(--gap-md);
    padding: var(--gap-sm) var(--gap-md);
    background: rgba(255, 82, 82, 0.08);
    border: 1px solid rgba(255, 82, 82, 0.15);
    border-radius: var(--radius-sm);
    font-size: 11px; color: var(--error);
    display: flex; align-items: center;
    justify-content: space-between; gap: var(--gap-sm);
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
    min-width: 280px;
    box-shadow: var(--shadow-lg);
  }
  .confirm-modal p { font-size: 13px; margin-bottom: var(--gap-md); }
  .confirm-actions {
    display: flex; gap: var(--gap-sm); justify-content: flex-end;
  }
</style>
