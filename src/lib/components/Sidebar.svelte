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
  import Icons from './Icons.svelte';
  import AboutModal from './AboutModal.svelte';

  export let onAddConnection: () => void = () => {};

  let confirmDisconnectId: string | null = null;
  let confirmDisconnectName: string = '';
  let appVersion = '';
  let showAboutModal = false;

  onMount(async () => {
    try {
      appVersion = await getVersion();
    } catch (e) {
      console.warn('Failed to get app version:', e);
    }
  });

  $: uptime = $serverInfo ? formatUptime(parseInt($serverInfo.uptime_in_seconds)) : '';
  $: formattedTotalKeys = $serverInfo?.total_keys ? formatNumber($serverInfo.total_keys) : '0';

  function formatNumber(str: string): string {
    const num = parseInt(str);
    if (isNaN(num)) return str;
    return num.toLocaleString();
  }

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

  function getEnvColor(name: string): string {
    const n = name.toLowerCase();
    if (n.includes('prod')) return '#ef4444';
    if (n.includes('uat') || n.includes('staging')) return '#f59e0b';
    if (n.includes('dev') || n.includes('local')) return '#10b981';
    return '#06b6d4';
  }
</script>

<div class="sidebar">
  <div class="sidebar-header">
    <div class="app-logo">
      <div class="logo-icon-wrap">
        <img src="/favicon.png" alt="Redis Manager" class="app-icon-img" />
      </div>
      <div class="logo-text-wrap">
        <span class="app-title">Redis Manager</span>
        {#if appVersion}
          <span class="app-ver">v{appVersion}</span>
        {/if}
      </div>
    </div>
  </div>

  <div class="sidebar-body">
    <!-- Connections Section -->
    {#if $connectionTabs.length > 0}
      <div class="conn-section">
        <div class="section-header">
          <div class="section-title-wrap">
            <span class="section-label">Connections</span>
            <span class="conn-count">{$connectionTabs.length}</span>
          </div>
          <button class="btn-add" on:click={onAddConnection} title="Add another Redis connection">
            <Icons name="plus" size={13} />
          </button>
        </div>

        <div class="conn-list">
          {#each $connectionTabs as tab (tab.id)}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              class="conn-card"
              class:active={tab.is_active}
              on:click={() => handleSwitch(tab.id)}
            >
              <div class="card-indicator" style="background: {tab.is_active ? getEnvColor(tab.name) : 'transparent'}"></div>
              <div class="card-content">
                <div class="card-top">
                  <span
                    class="card-dot"
                    class:active={tab.is_active}
                    style="--dot-color: {getEnvColor(tab.name)}"
                  ></span>
                  <span class="card-name" title={tab.name}>{tab.name}</span>
                  <span class="card-mode badge {tab.mode === 'cluster' ? 'badge-hash' : 'badge-string'}">
                    {tab.mode === 'cluster' ? 'CLUSTER' : 'STANDALONE'}
                  </span>
                </div>
                <div class="card-bottom">
                  <span class="card-status">{tab.is_active ? '● Active' : 'Connected'}</span>
                  <span class="card-url truncate" title={tab.url}>{tab.url}</span>
                </div>
              </div>
              <button
                class="card-close"
                on:click|stopPropagation={() => askDisconnect(tab.id, tab.name)}
                title="Disconnect"
              >
                <Icons name="close" size={12} />
              </button>
            </div>
          {/each}
        </div>
      </div>
    {/if}

    <!-- Server Info Metrics -->
    {#if $isConnected && $serverInfo}
      <div class="server-info animate-fade">
        <div class="info-header">
          <span class="info-title">SERVER STATS</span>
          <span class="badge { $serverInfo.mode === 'cluster' ? 'badge-hash' : 'badge-string' }">
            {$serverInfo.mode.toUpperCase()}
          </span>
        </div>

        <div class="info-grid">
          <div class="info-item">
            <span class="info-label">Version</span>
            <span class="info-value mono">{$serverInfo.version}</span>
          </div>
          <div class="info-item">
            <span class="info-label">Memory</span>
            <span class="info-value mono text-accent">{$serverInfo.used_memory_human}</span>
          </div>
          <div class="info-item">
            <span class="info-label">Total Keys</span>
            <span class="info-value mono text-success">{formattedTotalKeys}</span>
          </div>
          <div class="info-item">
            <span class="info-label">Clients</span>
            <span class="info-value mono">{$serverInfo.connected_clients}</span>
          </div>
          <div class="info-item full-width">
            <span class="info-label">Uptime</span>
            <span class="info-value mono">{uptime}</span>
          </div>
        </div>
      </div>
    {/if}

    {#if $error}
      <div class="error-banner animate-fade">
        <div class="error-msg truncate">⚠ {$error}</div>
        <button class="btn btn-sm btn-icon" on:click={() => error.set(null)}>✕</button>
      </div>
    {/if}
  </div>

  <!-- Footer with author information -->
  <div class="sidebar-footer">
    <button class="btn-author" on:click={() => showAboutModal = true} title="Xem thông tin tác giả">
      <div class="author-mini-card">
        <div class="author-avatar-mini">MD</div>
        <div class="author-meta-mini">
          <span class="author-name-text">Ngô Minh Đức</span>
          <span class="author-sub-text">Tác giả • About</span>
        </div>
      </div>
      <div class="author-info-icon">
        <Icons name="info" size={14} />
      </div>
    </button>
  </div>

  <!-- Confirm Disconnect Modal -->
  {#if confirmDisconnectId}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="confirm-overlay" on:click={() => confirmDisconnectId = null}>
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="confirm-modal animate-fade" on:click|stopPropagation>
        <p>Disconnect <strong>{confirmDisconnectName}</strong>?</p>
        <div class="confirm-actions">
          <button class="btn btn-sm" on:click={() => confirmDisconnectId = null}>Cancel</button>
          <button class="btn btn-sm btn-danger" on:click={doDisconnect}>Disconnect</button>
        </div>
      </div>
    </div>
  {/if}

  {#if showAboutModal}
    <AboutModal onClose={() => showAboutModal = false} />
  {/if}
</div>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-secondary);
    border-right: 1px solid var(--border-primary);
    overflow: hidden;
  }

  .sidebar-body {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }

  .sidebar-header {
    padding: var(--gap-md) var(--gap-md);
    border-bottom: 1px solid var(--border-primary);
    flex-shrink: 0;
  }
  .app-logo {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .logo-icon-wrap {
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .app-icon-img {
    width: 26px;
    height: 26px;
    border-radius: 6px;
    object-fit: cover;
    box-shadow: 0 2px 8px rgba(0, 212, 255, 0.25);
  }
  .logo-text-wrap {
    display: flex;
    flex-direction: column;
  }
  .app-title {
    font-size: 14px;
    font-weight: 700;
    letter-spacing: -0.2px;
    background: linear-gradient(135deg, var(--accent), #c084fc);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
  }
  .app-ver {
    font-size: 10px;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  /* Connection section */
  .conn-section {
    border-bottom: 1px solid var(--border-primary);
    flex-shrink: 0;
  }
  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--gap-sm) var(--gap-md);
  }
  .section-title-wrap {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .section-label {
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.6px;
    color: var(--text-muted);
  }
  .conn-count {
    font-size: 9px;
    font-weight: 700;
    background: rgba(255, 255, 255, 0.08);
    color: var(--text-secondary);
    padding: 1px 6px;
    border-radius: 999px;
  }
  .btn-add {
    background: none;
    border: 1px solid var(--border-primary);
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    cursor: pointer;
    padding: 2px 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all var(--transition-fast);
  }
  .btn-add:hover {
    color: var(--accent);
    border-color: var(--accent);
    background: rgba(0, 212, 255, 0.08);
  }

  .conn-list {
    padding: 0 var(--gap-xs) var(--gap-sm);
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  /* Connection card */
  .conn-card {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 7px 8px;
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
    box-shadow: inset 0 0 12px rgba(0, 212, 255, 0.05);
  }

  .card-indicator {
    width: 3px;
    height: 30px;
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
    gap: 6px;
  }
  .card-dot {
    width: 6px;
    height: 6px;
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
    font-weight: 600;
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
    gap: 8px;
    padding-left: 12px;
    font-size: 10px;
  }
  .card-status {
    color: var(--text-accent);
    font-weight: 500;
  }
  .card-url {
    color: var(--text-muted);
    font-family: var(--font-mono);
    font-size: 10px;
    flex: 1;
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
  .conn-card:hover .card-close {
    opacity: 1;
  }
  .card-close:hover {
    color: var(--error);
    background: rgba(255, 82, 82, 0.12);
  }

  /* Server Info */
  .server-info {
    padding: var(--gap-md) var(--gap-md);
    border-bottom: 1px solid var(--border-primary);
    flex-shrink: 0;
  }
  .info-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
  }
  .info-title {
    font-size: 10px;
    font-weight: 700;
    color: var(--text-muted);
    letter-spacing: 0.6px;
  }
  .info-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  .info-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
    background: var(--bg-primary);
    padding: 6px 8px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-primary);
  }
  .info-item.full-width {
    grid-column: span 2;
  }
  .info-label {
    font-size: 9px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  .info-value {
    font-size: 11px;
    font-weight: 600;
  }

  .error-banner {
    margin: var(--gap-sm) var(--gap-md);
    padding: var(--gap-sm) var(--gap-md);
    background: rgba(255, 82, 82, 0.1);
    border: 1px solid rgba(255, 82, 82, 0.25);
    border-radius: var(--radius-sm);
    font-size: 11px;
    color: var(--error);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--gap-sm);
  }
  .error-msg {
    flex: 1;
  }

  /* Confirm modal */
  .confirm-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
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
  .confirm-modal p {
    font-size: 13px;
    margin-bottom: var(--gap-md);
  }
  .confirm-actions {
    display: flex;
    gap: var(--gap-sm);
    justify-content: flex-end;
  }

  /* Footer with author info */
  .sidebar-footer {
    padding: 8px var(--gap-md);
    border-top: 1px solid var(--border-primary);
    background: var(--bg-primary);
    flex-shrink: 0;
  }

  .btn-author {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 10px;
    background: var(--bg-secondary);
    border: 1px solid var(--border-secondary);
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    cursor: pointer;
    transition: all var(--transition-fast);
  }
  .btn-author:hover {
    background: var(--bg-hover);
    border-color: var(--border-accent);
  }

  .author-mini-card {
    display: flex;
    align-items: center;
    gap: 8px;
    text-align: left;
  }
  .author-avatar-mini {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: linear-gradient(135deg, #ef4444, #38bdf8);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 10px;
    font-weight: 700;
    color: #fff;
    flex-shrink: 0;
    box-shadow: 0 1px 4px rgba(0, 212, 255, 0.3);
  }
  .author-meta-mini {
    display: flex;
    flex-direction: column;
    line-height: 1.2;
  }
  .author-name-text {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-primary);
  }
  .author-sub-text {
    font-size: 9px;
    color: var(--text-muted);
  }
  .author-info-icon {
    color: var(--text-muted);
    display: flex;
    align-items: center;
    transition: color var(--transition-fast);
  }
  .btn-author:hover .author-info-icon {
    color: var(--accent);
  }
</style>
