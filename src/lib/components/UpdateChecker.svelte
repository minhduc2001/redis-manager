<script lang="ts">
  import { onMount } from 'svelte';
  import { check } from '@tauri-apps/plugin-updater';
  import { relaunch } from '@tauri-apps/plugin-process';

  let updateAvailable = false;
  let updateVersion = '';
  let updateNotes = '';
  let downloading = false;
  let downloadProgress = 0;
  let downloadTotal = 0;
  let installed = false;
  let checkError = '';
  let showBanner = false;
  let dismissed = false;

  // Store the update object
  let pendingUpdate: any = null;

  onMount(async () => {
    // Check for updates 3 seconds after app starts
    setTimeout(() => checkForUpdates(), 3000);
  });

  async function checkForUpdates() {
    try {
      checkError = '';
      const update = await check();
      if (update) {
        pendingUpdate = update;
        updateAvailable = true;
        updateVersion = update.version;
        updateNotes = update.body || '';
        showBanner = true;
      }
    } catch (e: any) {
      console.warn('Update check failed:', e);
      checkError = e.toString();
    }
  }

  async function startUpdate() {
    if (!pendingUpdate) return;
    downloading = true;
    downloadProgress = 0;
    downloadTotal = 0;

    try {
      await pendingUpdate.downloadAndInstall((event: any) => {
        switch (event.event) {
          case 'Started':
            downloadTotal = event.data.contentLength || 0;
            break;
          case 'Progress':
            downloadProgress += event.data.chunkLength;
            break;
          case 'Finished':
            installed = true;
            break;
        }
      });

      // Relaunch after install
      await relaunch();
    } catch (e: any) {
      checkError = e.toString();
      downloading = false;
    }
  }

  function dismiss() {
    dismissed = true;
    showBanner = false;
  }

  $: progressPercent = downloadTotal > 0
    ? Math.round((downloadProgress / downloadTotal) * 100)
    : 0;
</script>

{#if showBanner && !dismissed}
  <div class="update-banner animate-fade">
    <div class="update-content">
      {#if downloading}
        <div class="update-progress">
          <div class="progress-info">
            <span class="update-icon">⬇️</span>
            <span>Downloading v{updateVersion}...</span>
            {#if downloadTotal > 0}
              <span class="progress-pct">{progressPercent}%</span>
            {/if}
          </div>
          <div class="progress-bar">
            <div class="progress-fill" style="width: {progressPercent}%"></div>
          </div>
        </div>
      {:else}
        <div class="update-info">
          <span class="update-icon">🚀</span>
          <span class="update-text">
            <strong>v{updateVersion}</strong> is available!
            {#if updateNotes}
              <span class="update-notes">{updateNotes}</span>
            {/if}
          </span>
        </div>
        <div class="update-actions">
          <button class="btn btn-sm btn-primary update-btn" on:click={startUpdate}>
            ⬆ Update Now
          </button>
          <button class="btn btn-sm dismiss-btn" on:click={dismiss}>
            Later
          </button>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .update-banner {
    position: fixed;
    bottom: 16px;
    right: 16px;
    z-index: 9999;
    max-width: 380px;
    width: 100%;
    background: var(--bg-secondary);
    border: 1px solid var(--border-accent);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg), 0 0 20px rgba(0, 212, 255, 0.1);
    overflow: hidden;
  }

  .update-content {
    padding: var(--gap-md) var(--gap-lg);
  }

  .update-info {
    display: flex;
    align-items: flex-start;
    gap: var(--gap-sm);
    margin-bottom: var(--gap-md);
  }
  .update-icon {
    font-size: 16px;
    flex-shrink: 0;
    line-height: 1;
  }
  .update-text {
    font-size: 12px;
    color: var(--text-primary);
    line-height: 1.5;
  }
  .update-text strong {
    color: var(--accent);
  }
  .update-notes {
    display: block;
    color: var(--text-secondary);
    font-size: 11px;
    margin-top: 2px;
    max-height: 60px;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .update-actions {
    display: flex;
    gap: var(--gap-sm);
    justify-content: flex-end;
  }
  .update-btn {
    padding: 5px 14px;
    font-weight: 600;
  }
  .dismiss-btn {
    padding: 5px 10px;
    color: var(--text-muted);
    border-color: var(--border-primary);
  }
  .dismiss-btn:hover {
    color: var(--text-primary);
  }

  /* Progress */
  .update-progress {
    display: flex;
    flex-direction: column;
    gap: var(--gap-sm);
  }
  .progress-info {
    display: flex;
    align-items: center;
    gap: var(--gap-sm);
    font-size: 12px;
    color: var(--text-primary);
  }
  .progress-pct {
    color: var(--accent);
    font-weight: 600;
    font-family: var(--font-mono);
    margin-left: auto;
  }
  .progress-bar {
    height: 4px;
    background: var(--bg-tertiary);
    border-radius: 2px;
    overflow: hidden;
  }
  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, var(--accent), #a78bfa);
    border-radius: 2px;
    transition: width 0.3s ease;
  }
</style>
