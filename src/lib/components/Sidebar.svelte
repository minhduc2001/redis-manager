<script lang="ts">
  import { onMount } from 'svelte';
  import { getVersion } from '@tauri-apps/api/app';
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
  let appVersion = '';

  onMount(async () => {
    try {
      appVersion = await getVersion();
    } catch (e) {
      console.warn('Failed to get app version:', e);
    }
  });

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

  // Environment color based on name
  function getEnvColor(name: string): string {
    const n = name.toLowerCase();
    if (n.includes('prod')) return '#ef5350';
    if (n.includes('uat') || n.includes('staging')) return '#ffa726';
    if (n.includes('dev') || n.includes('local')) return '#66bb6a';
    return '#4fc3f7';
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
      <div style="display: flex; flex-direction: column; gap: 2px;">
        <span class="app-title">Redis Manager</span>
        {#if appVersion}
          <span style="font-size: 10px; color: var(--text-muted); line-height: 1; font-family: var(--font-mono);">v{appVersion}</span>
        {/if}
      </div>
    </div>
  </div>

  <!-- Connection Cards -->
  {#if $connectionTabs.length > 0}
    <div class="conn-section">
      <div class="section-header">
        <span class="section-label">Connections</span>
        <span class="conn-count">{$connectionTabs.length}</span>
        <button class="btn-add" on:click={onAddConnection} title="Add connection">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
            <path d="M12 5v14M5 12h14"/>
          </svg>
        </button>
      </div>
      <div class="conn-list">
        {#each $connectionTabs as tab (tab.id)}
          <div
            class="conn-card"
            class:active={tab.is_active}
            on:click={() => handleSwitch(tab.id)}
          >
            <div class="card-indicator" style="background: {tab.is_active ? getEnvColor(tab.name) : 'transparent'}"></div>
            <div class="card-content">
              <div class="card-top">
                <span class="card-dot" class:active={tab.is_active} style="--dot-color: {getEnvColor(tab.name)}"></span>
                <span class="card-name">{tab.name}</span>
                <span class="card-mode badge {tab.mode === 'cluster' ? 'badge-hash' : 'badge-string'}">{tab.mode}</span>
              </div>
              <div class="card-bottom">
                <span class="card-status">{tab.is_active ? 'Active' : 'Connected'}</span>
              </div>
            </div>
            <button
              class="card-close"
              on:click|stopPropagation={() => askDisconnect(tab.id, tab.name)}
              title="Disconnect"
            >
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                <path d="M18 6L6 18M6 6l12 12"/>
              </svg>
            </button>
          </div>
        {/each}
      </div>
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
    overflow-y: auto;
  }
  .sidebar-header {
    padding: var(--gap-md) var(--gap-lg);
    border-bottom: 1px solid var(--border-primary);
    flex-shrink: 0;
  }
  .app-logo { display: flex; align-items: center; gap: var(--gap-sm); }
  .app-title {
    font-size: 14px; font-weight: 700;
    background: linear-gradient(135deg, var(--accent), #a78bfa);
    -webkit-background-clip: text; -webkit-text-fill-color: transparent;
    background-clip: text;
  }

  /* Connection section */
  .conn-section {
    border-bottom: 1px solid var(--border-primary);
    flex-shrink: 0;
  }
  .section-header {
    display: flex;
    align-items: center;
    gap: var(--gap-sm);
    padding: var(--gap-sm) var(--gap-md);
  }
  .section-label {
    font-size: 10px; font-weight: 600;
    text-transform: uppercase; letter-spacing: 0.5px;
    color: var(--text-muted);
    flex: 1;
  }
  .conn-count {
    font-size: 9px;
    font-weight: 700;
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-muted);
    padding: 1px 6px;
    border-radius: 8px;
  }
  .btn-add {
    background: none;
    border: 1px solid var(--border-primary);
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    cursor: pointer;
    padding: 3px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all var(--transition-fast);
  }
  .btn-add:hover {
    color: var(--accent);
    border-color: var(--accent);
    background: rgba(0, 212, 255, 0.06);
  }

  .conn-list {
    padding: 0 var(--gap-sm) var(--gap-sm);
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  /* Connection card */
  .conn-card {
    display: flex;
    align-items: center;
    gap: var(--gap-sm);
    padding: 8px var(--gap-sm);
    cursor: pointer;
    transition: all var(--transition-fast);
    border-radius: var(--radius-sm);
    position: relative;
    overflow: hidden;
  }
  .conn-card:hover {
    background: var(--bg-hover);
  }
  .conn-card.active {
    background: var(--bg-active);
  }

  .card-indicator {
    width: 3px;
    height: 28px;
    border-radius: 2px;
    flex-shrink: 0;
    transition: background var(--transition-fast);
  }

  .card-content {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .card-top {
    display: flex;
    align-items: center;
    gap: var(--gap-xs);
  }
  .card-dot {
    width: 7px; height: 7px;
    border-radius: 50%;
    background: var(--text-muted);
    flex-shrink: 0;
    transition: all var(--transition-fast);
  }
  .card-dot.active {
    background: var(--dot-color, var(--success));
    box-shadow: 0 0 6px var(--dot-color, var(--success));
  }
  .card-name {
    font-size: 12px;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
    min-width: 0;
  }
  .card-mode {
    font-size: 8px;
    padding: 1px 5px;
    flex-shrink: 0;
  }
  .card-bottom {
    display: flex;
    align-items: center;
    gap: var(--gap-sm);
    padding-left: 11px; /* align with text after dot */
  }
  .card-status {
    font-size: 10px;
    color: var(--text-muted);
  }

  .card-close {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 4px;
    border-radius: 4px;
    opacity: 0;
    transition: all var(--transition-fast);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .conn-card:hover .card-close { opacity: 1; }
  .card-close:hover {
    color: var(--error);
    background: rgba(255, 82, 82, 0.1);
  }

  /* Server info */
  .server-info {
    padding: var(--gap-md) var(--gap-lg);
    border-bottom: 1px solid var(--border-primary);
    flex-shrink: 0;
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
