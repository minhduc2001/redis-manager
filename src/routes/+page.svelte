<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { isConnected, autoReconnectLast } from '$lib/stores/redis';
  import ConnectionForm from '$lib/components/ConnectionForm.svelte';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import KeyBrowser from '$lib/components/KeyBrowser.svelte';
  import KeyDetail from '$lib/components/KeyDetail.svelte';
  import Console from '$lib/components/Console.svelte';
  import UpdateChecker from '$lib/components/UpdateChecker.svelte';

  let showAddConnection = false;
  let rightPanel: 'detail' | 'console' = 'detail';

  // Resizable panel widths
  let sidebarWidth = 220;
  let keyBrowserWidth = 320;
  let containerEl: HTMLDivElement;
  let isDragging: 'sidebar' | 'keybrowser' | null = null;

  const MIN_SIDEBAR = 160;
  const MAX_SIDEBAR = 360;
  const MIN_KEYBROWSER = 240;
  const MAX_KEYBROWSER = 600;

  onMount(() => {
    autoReconnectLast();

    // Load saved widths
    try {
      const saved = localStorage.getItem('redis-manager-panel-widths');
      if (saved) {
        const { sidebar, keybrowser } = JSON.parse(saved);
        if (sidebar) sidebarWidth = Math.max(MIN_SIDEBAR, Math.min(MAX_SIDEBAR, sidebar));
        if (keybrowser) keyBrowserWidth = Math.max(MIN_KEYBROWSER, Math.min(MAX_KEYBROWSER, keybrowser));
      }
    } catch {}
  });

  function saveWidths() {
    try {
      localStorage.setItem('redis-manager-panel-widths', JSON.stringify({
        sidebar: sidebarWidth,
        keybrowser: keyBrowserWidth,
      }));
    } catch {}
  }

  function handleMouseDown(panel: 'sidebar' | 'keybrowser') {
    isDragging = panel;
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
  }

  function handleMouseMove(e: MouseEvent) {
    if (!isDragging || !containerEl) return;
    const containerRect = containerEl.getBoundingClientRect();
    const x = e.clientX - containerRect.left;

    if (isDragging === 'sidebar') {
      sidebarWidth = Math.max(MIN_SIDEBAR, Math.min(MAX_SIDEBAR, x));
    } else if (isDragging === 'keybrowser') {
      const newWidth = x - sidebarWidth;
      keyBrowserWidth = Math.max(MIN_KEYBROWSER, Math.min(MAX_KEYBROWSER, newWidth));
    }
  }

  function handleMouseUp() {
    isDragging = null;
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
    window.removeEventListener('mousemove', handleMouseMove);
    window.removeEventListener('mouseup', handleMouseUp);
    saveWidths();
  }

  function handleAddConnection() {
    showAddConnection = true;
  }

  function handleConnected() {
    showAddConnection = false;
  }
</script>

<UpdateChecker />

<div class="app">
  {#if !$isConnected}
    <ConnectionForm onConnected={handleConnected} />
  {:else}
    <div
      class="main-layout"
      bind:this={containerEl}
      style="grid-template-columns: {sidebarWidth}px 4px {keyBrowserWidth}px 4px 1fr"
    >
      <aside class="panel-sidebar">
        <Sidebar onAddConnection={handleAddConnection} />
      </aside>

      <!-- Sidebar Resize Handle -->
      <div
        class="resize-handle"
        class:active={isDragging === 'sidebar'}
        on:mousedown={() => handleMouseDown('sidebar')}
      >
        <div class="resize-line"></div>
      </div>

      <section class="panel-keys">
        <KeyBrowser />
      </section>

      <!-- Key Browser Resize Handle -->
      <div
        class="resize-handle"
        class:active={isDragging === 'keybrowser'}
        on:mousedown={() => handleMouseDown('keybrowser')}
      >
        <div class="resize-line"></div>
      </div>

      <section class="panel-right">
        <div class="panel-tabs">
          <button class="panel-tab" class:active={rightPanel === 'detail'} on:click={() => rightPanel = 'detail'}>
            📋 Key Detail
          </button>
          <button class="panel-tab" class:active={rightPanel === 'console'} on:click={() => rightPanel = 'console'}>
            ⌨ CLI Console
          </button>
        </div>
        <div class="panel-content">
          {#if rightPanel === 'detail'}
            <KeyDetail />
          {:else}
            <Console />
          {/if}
        </div>
      </section>
    </div>

    {#if showAddConnection}
      <div class="modal-overlay" on:click={() => showAddConnection = false}>
        <div class="modal-content" on:click|stopPropagation>
          <div class="modal-header">
            <h3>Add Connection</h3>
            <button class="btn btn-sm btn-icon" on:click={() => showAddConnection = false}>✕</button>
          </div>
          <ConnectionForm onConnected={handleConnected} />
        </div>
      </div>
    {/if}
  {/if}
</div>

<style>
  .app {
    height: 100vh;
    overflow: hidden;
  }
  .main-layout {
    display: grid;
    height: 100vh;
    overflow: hidden;
  }
  .panel-sidebar { overflow: hidden; }
  .panel-keys {
    overflow: hidden;
    background: var(--bg-secondary);
  }
  .panel-right {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--bg-primary);
  }

  /* Resize handle — VS Code style */
  .resize-handle {
    width: 4px;
    cursor: col-resize;
    background: transparent;
    position: relative;
    z-index: 10;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background var(--transition-fast);
  }
  .resize-handle:hover,
  .resize-handle.active {
    background: var(--accent);
  }
  .resize-line {
    width: 1px;
    height: 100%;
    background: var(--border-primary);
    transition: background var(--transition-fast);
  }
  .resize-handle:hover .resize-line,
  .resize-handle.active .resize-line {
    background: transparent;
  }

  /* Panel tabs */
  .panel-tabs {
    display: flex;
    border-bottom: 1px solid var(--border-primary);
    background: var(--bg-secondary);
    flex-shrink: 0;
  }
  .panel-tab {
    flex: 1;
    background: none;
    border: none;
    padding: 8px 16px;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-muted);
    cursor: pointer;
    border-bottom: 2px solid transparent;
    transition: all var(--transition-fast);
    font-family: var(--font-sans);
  }
  .panel-tab:hover {
    color: var(--text-primary);
    background: var(--bg-hover);
  }
  .panel-tab.active {
    color: var(--accent);
    border-bottom-color: var(--accent);
    background: var(--bg-primary);
  }
  .panel-content {
    flex: 1;
    overflow: hidden;
  }

  /* Modal */
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }
  .modal-content {
    background: var(--bg-secondary);
    border: 1px solid var(--border-secondary);
    border-radius: var(--radius-xl);
    overflow: hidden;
    box-shadow: var(--shadow-lg);
    max-width: 520px;
    width: 100%;
  }
  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--gap-lg) var(--gap-xl) 0;
  }
  .modal-header h3 { font-size: 16px; font-weight: 600; }
  .modal-content :global(.connection-page) { height: auto; min-height: 0; }
  .modal-content :global(.connection-container) { box-shadow: none; border: none; border-radius: 0; }
  .modal-content :global(.logo-section) { display: none; }
</style>
