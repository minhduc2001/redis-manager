<script lang="ts">
  import '../app.css';
  import { isConnected } from '$lib/stores/redis';
  import ConnectionForm from '$lib/components/ConnectionForm.svelte';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import KeyBrowser from '$lib/components/KeyBrowser.svelte';
  import KeyDetail from '$lib/components/KeyDetail.svelte';
  import Console from '$lib/components/Console.svelte';

  let showAddConnection = false;
  let rightPanel: 'detail' | 'console' = 'detail';

  function handleAddConnection() {
    showAddConnection = true;
  }

  function handleConnected() {
    showAddConnection = false;
  }
</script>

<div class="app">
  {#if !$isConnected}
    <ConnectionForm onConnected={handleConnected} />
  {:else}
    <div class="main-layout">
      <aside class="panel-sidebar">
        <Sidebar onAddConnection={handleAddConnection} />
      </aside>
      <section class="panel-keys">
        <KeyBrowser />
      </section>
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
    grid-template-columns: 220px 320px 1fr;
    height: 100vh;
    overflow: hidden;
  }
  .panel-sidebar { overflow: hidden; }
  .panel-keys {
    border-right: 1px solid var(--border-primary);
    overflow: hidden;
    background: var(--bg-secondary);
  }
  .panel-right {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--bg-primary);
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
