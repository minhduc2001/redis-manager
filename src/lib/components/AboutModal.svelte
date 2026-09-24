<script lang="ts">
  import { onMount } from 'svelte';
  import { getVersion } from '@tauri-apps/api/app';
  import { openUrl } from '@tauri-apps/plugin-opener';

  export let onClose: () => void = () => {};

  let appVersion = '0.1.7';

  onMount(async () => {
    try {
      appVersion = await getVersion();
    } catch {}
  });

  async function openLink(url: string) {
    try {
      await openUrl(url);
    } catch {
      window.open(url, '_blank');
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onClose();
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="modal-overlay" on:click={onClose}>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="about-card animate-fade" on:click|stopPropagation>
    <div class="about-header">
      <div class="app-branding">
        <div class="app-icon-wrap">
          <img src="/favicon.png" alt="Redis Manager Logo" class="app-icon" />
        </div>
        <div class="app-title-group">
          <div class="title-row">
            <h2>Redis Manager</h2>
            <span class="version-tag">v{appVersion}</span>
          </div>
          <p class="tagline">Lightweight, ultra-fast Redis desktop GUI</p>
        </div>
      </div>
      <button class="btn-close" on:click={onClose} title="Close (Esc)">✕</button>
    </div>

    <div class="about-body">
      <!-- Author Section -->
      <div class="author-section">
        <div class="section-title">THÔNG TIN TÁC GIẢ / AUTHOR</div>
        <div class="author-card">
          <div class="avatar-circle">
            <span>MD</span>
          </div>
          <div class="author-details">
            <div class="author-name">Ngô Minh Đức</div>
            <div class="author-handle">@minhduc2001 (ducnm)</div>
            <div class="author-email">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <rect width="20" height="16" x="2" y="4" rx="2"/><path d="m22 7-8.97 5.7a1.94 1.94 0 0 1-2.06 0L2 7"/>
              </svg>
              <span>ngoduc2468@gmail.com</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Links & Actions -->
      <div class="links-section">
        <button
          type="button"
          class="btn-social github-btn"
          on:click={() => openLink('https://github.com/minhduc2001/redis-manager')}
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
            <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0 0 24 12c0-6.63-5.37-12-12-12z"/>
          </svg>
          <span>GitHub Repository</span>
          <span class="external-icon">↗</span>
        </button>

        <button
          type="button"
          class="btn-social issue-btn"
          on:click={() => openLink('https://github.com/minhduc2001/redis-manager/issues')}
        >
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10"/><line x1="12" x2="12" y1="8" y2="12"/><line x1="12" x2="12.01" y1="16" y2="16"/>
          </svg>
          <span>Báo cáo sự cố / Góp ý</span>
          <span class="external-icon">↗</span>
        </button>
      </div>

      <!-- Tech Stack -->
      <div class="tech-section">
        <div class="section-title">CÔNG NGHỆ SỬ DỤNG</div>
        <div class="tech-tags">
          <span class="tech-tag">Tauri v2</span>
          <span class="tech-tag">Svelte 5</span>
          <span class="tech-tag">Rust</span>
          <span class="tech-tag">redis-rs</span>
          <span class="tech-tag">Tokio Async</span>
          <span class="tech-tag">Vite</span>
          <span class="tech-tag">TypeScript</span>
        </div>
      </div>
    </div>

    <div class="about-footer">
      <span class="license">MIT License • Free & Open Source</span>
      <span class="copyright">© 2026 Ngô Minh Đức. All rights reserved.</span>
    </div>
  </div>
</div>

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.75);
    backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .about-card {
    background: var(--bg-secondary);
    border: 1px solid var(--border-secondary);
    border-radius: var(--radius-xl);
    width: 90%;
    max-width: 460px;
    box-shadow: 0 20px 48px rgba(0, 0, 0, 0.7), 0 0 32px rgba(0, 212, 255, 0.1);
    overflow: hidden;
  }

  .about-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--gap-lg) var(--gap-xl);
    border-bottom: 1px solid var(--border-primary);
    background: rgba(19, 19, 48, 0.5);
  }

  .app-branding {
    display: flex;
    align-items: center;
    gap: var(--gap-md);
  }

  .app-icon-wrap {
    width: 44px;
    height: 44px;
    border-radius: 12px;
    overflow: hidden;
    box-shadow: 0 4px 16px rgba(239, 68, 68, 0.25), 0 0 12px rgba(0, 212, 255, 0.2);
    border: 1px solid rgba(255, 255, 255, 0.15);
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-primary);
  }

  .app-icon {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .app-title-group {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .title-row h2 {
    font-size: 16px;
    font-weight: 700;
    letter-spacing: -0.3px;
    background: linear-gradient(135deg, #fff, #38bdf8);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
  }

  .version-tag {
    font-size: 10px;
    font-weight: 700;
    color: var(--accent);
    background: rgba(0, 212, 255, 0.12);
    border: 1px solid rgba(0, 212, 255, 0.3);
    padding: 1px 6px;
    border-radius: 999px;
    font-family: var(--font-mono);
  }

  .tagline {
    font-size: 11px;
    color: var(--text-muted);
  }

  .btn-close {
    background: none;
    border: none;
    color: var(--text-muted);
    font-size: 14px;
    cursor: pointer;
    padding: 6px 10px;
    border-radius: var(--radius-sm);
    transition: all var(--transition-fast);
  }
  .btn-close:hover {
    color: var(--text-primary);
    background: var(--bg-hover);
  }

  .about-body {
    padding: var(--gap-lg) var(--gap-xl);
    display: flex;
    flex-direction: column;
    gap: var(--gap-lg);
  }

  .section-title {
    font-size: 9px;
    font-weight: 700;
    color: var(--text-muted);
    letter-spacing: 0.8px;
    margin-bottom: 6px;
  }

  .author-card {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px;
    background: var(--bg-primary);
    border: 1px solid var(--border-secondary);
    border-radius: var(--radius-md);
  }

  .avatar-circle {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background: linear-gradient(135deg, #ef4444, #38bdf8);
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 700;
    font-size: 14px;
    color: #fff;
    box-shadow: 0 2px 10px rgba(0, 212, 255, 0.25);
    flex-shrink: 0;
  }

  .author-details {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .author-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .author-handle {
    font-size: 11px;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  .author-email {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    color: var(--text-secondary);
    margin-top: 2px;
  }

  .links-section {
    display: flex;
    gap: 8px;
  }

  .btn-social {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 8px 12px;
    border-radius: var(--radius-sm);
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: all var(--transition-fast);
    border: 1px solid var(--border-secondary);
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .github-btn:hover {
    background: #1f2937;
    border-color: #4b5563;
    color: #fff;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.4);
  }

  .issue-btn:hover {
    background: var(--bg-hover);
    border-color: var(--border-accent);
    color: var(--accent);
  }

  .external-icon {
    font-size: 12px;
    opacity: 0.6;
  }

  .tech-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
  }

  .tech-tag {
    font-size: 10px;
    font-weight: 500;
    background: var(--bg-primary);
    border: 1px solid var(--border-primary);
    color: var(--text-secondary);
    padding: 2px 8px;
    border-radius: 4px;
    font-family: var(--font-mono);
  }

  .about-footer {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: var(--gap-md) var(--gap-xl);
    border-top: 1px solid var(--border-primary);
    background: rgba(7, 7, 20, 0.6);
    font-size: 10px;
    color: var(--text-muted);
  }
</style>
