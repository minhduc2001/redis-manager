import { writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import type {
  ConnectionInfo,
  ConnectionTab,
  ServerInfo,
  KeyEntry,
  ScanResult,
  KeyDetail,
  SavedConnection,
} from './types';

// Connection state
export const isConnected = writable(false);
export const connectionTabs = writable<ConnectionTab[]>([]);
export const activeConnectionId = writable<string | null>(null);
export const serverInfo = writable<ServerInfo | null>(null);

// Keys state
export const keys = writable<KeyEntry[]>([]);
export const scanCursor = writable(0);
export const hasMore = writable(true);
export const isLoading = writable(false);
export const searchPattern = writable('*');
export const selectedKey = writable<string | null>(null);
export const selectedKeys = writable<Set<string>>(new Set());
export const keyDetail = writable<KeyDetail | null>(null);
export const isLoadingDetail = writable(false);

// Error state
export const error = writable<string | null>(null);

// Saved connections
export const savedConnections = writable<SavedConnection[]>(loadSavedConnections());

function loadSavedConnections(): SavedConnection[] {
  try {
    const data = localStorage.getItem('redis-manager-connections');
    return data ? JSON.parse(data) : [];
  } catch {
    return [];
  }
}

export function saveConnection(conn: SavedConnection) {
  savedConnections.update((list) => {
    // Deduplicate by URL (not ID) to avoid duplicates when clicking saved + connecting
    const idx = list.findIndex((c) => c.url === conn.url);
    if (idx >= 0) {
      list[idx] = { ...list[idx], name: conn.name, password: conn.password };
    } else {
      list.push(conn);
    }
    localStorage.setItem('redis-manager-connections', JSON.stringify(list));
    return [...list];
  });
}

export function removeConnection(id: string) {
  savedConnections.update((list) => {
    const removed = list.find((c) => c.id === id);
    const filtered = list.filter((c) => c.id !== id);
    localStorage.setItem('redis-manager-connections', JSON.stringify(filtered));
    // Remove from active-names list if present
    if (removed) {
      try {
        const activeNames: string[] = JSON.parse(localStorage.getItem('redis-manager-active-names') || '[]');
        const updated = activeNames.filter((n) => n !== removed.name);
        localStorage.setItem('redis-manager-active-names', JSON.stringify(updated));
      } catch {}
    }
    return filtered;
  });
}

function generateId() {
  return Date.now().toString(36) + Math.random().toString(36).substr(2);
}

// Actions
export async function connectRedis(url: string, password?: string, name?: string) {
  try {
    isLoading.set(true);
    error.set(null);
    const id = generateId();
    const connName = name || url.split(',')[0].split(':')[0] || 'Redis';
    const info = await invoke<ConnectionInfo>('connect_redis', {
      id,
      name: connName,
      url,
      password: password || null,
    });
    isConnected.set(true);
    activeConnectionId.set(info.id);

    // Refresh tab list
    await refreshConnectionTabs();

    // Load server info
    try {
      const sInfo = await invoke<ServerInfo>('get_server_info');
      serverInfo.set(sInfo);
    } catch (e) {
      console.warn('Failed to get server info:', e);
    }

    // Load initial keys
    await loadKeys('*', true);

    // Save active connections list
    saveActiveConnections();

    return info;
  } catch (e: any) {
    error.set(e.toString());
    throw e;
  } finally {
    isLoading.set(false);
  }
}

function saveActiveConnections() {
  // Read saved connections and mark which ones are currently active
  try {
    const saved = JSON.parse(localStorage.getItem('redis-manager-connections') || '[]');
    // Get current active connection tabs
    let tabs: ConnectionTab[] = [];
    const unsub = connectionTabs.subscribe((v) => (tabs = v));
    unsub();
    const activeUrls = tabs.map((t) => t.name);
    localStorage.setItem('redis-manager-active-names', JSON.stringify(activeUrls));
  } catch {}
}

export async function autoReconnectLast(): Promise<boolean> {
  const savedConns: SavedConnection[] = loadSavedConnections();
  const activeNames: string[] = (() => {
    try {
      return JSON.parse(localStorage.getItem('redis-manager-active-names') || '[]');
    } catch { return []; }
  })();

  if (activeNames.length === 0 || savedConns.length === 0) return false;

  let anySuccess = false;
  for (const name of activeNames) {
    const conn = savedConns.find((c) => c.name === name);
    if (!conn) continue;
    try {
      await connectRedis(conn.url, conn.password || undefined, conn.name);
      anySuccess = true;
    } catch {
      // Skip failed connections silently
    }
  }
  return anySuccess;
}

export async function disconnectRedis(id?: string) {
  try {
    let currentId = id;
    if (!currentId) {
      const unsub = activeConnectionId.subscribe((v) => (currentId = v || undefined));
      unsub();
    }
    if (!currentId) return;

    await invoke('disconnect_redis', { id: currentId });
    await refreshConnectionTabs();
    saveActiveConnections();

    // Check if there are remaining connections
    let tabs: ConnectionTab[] = [];
    const unsub = connectionTabs.subscribe((v) => (tabs = v));
    unsub();

    if (tabs.length === 0) {
      isConnected.set(false);
      activeConnectionId.set(null);
      serverInfo.set(null);
      keys.set([]);
      scanCursor.set(0);
      hasMore.set(true);
      selectedKey.set(null);
      selectedKeys.set(new Set());
      keyDetail.set(null);
      error.set(null);
      localStorage.removeItem('redis-manager-active-names');
    } else {
      // Switch to first available tab
      const newActive = tabs.find((t) => t.is_active) || tabs[0];
      await switchConnection(newActive.id);
    }
  } catch (e: any) {
    error.set(e.toString());
  }
}

export async function switchConnection(id: string) {
  try {
    isLoading.set(true);
    error.set(null);

    // Clear current data immediately to show loading state
    keys.set([]);
    scanCursor.set(0);
    hasMore.set(true);
    selectedKey.set(null);
    keyDetail.set(null);
    selectedKeys.set(new Set());
    serverInfo.set(null);

    await invoke('set_active_connection', { id });
    activeConnectionId.set(id);
    await refreshConnectionTabs();

    // Reload server info and keys for this connection
    try {
      const sInfo = await invoke<ServerInfo>('get_server_info');
      serverInfo.set(sInfo);
    } catch (e) {
      console.warn('Failed to get server info:', e);
    }

    selectedKey.set(null);
    keyDetail.set(null);
    selectedKeys.set(new Set());
    await loadKeys('*', true);
  } catch (e: any) {
    error.set(e.toString());
  } finally {
    isLoading.set(false);
  }
}

async function refreshConnectionTabs() {
  try {
    const tabs = await invoke<ConnectionTab[]>('get_connections');
    connectionTabs.set(tabs);
  } catch (e) {
    console.warn('Failed to refresh connections:', e);
  }
}

export async function testConnection(url: string, password?: string): Promise<boolean> {
  try {
    error.set(null);
    return await invoke<boolean>('test_connection', {
      url,
      password: password || null,
    });
  } catch (e: any) {
    error.set(e.toString());
    return false;
  }
}

export async function loadKeys(pattern?: string, reset = false) {
  try {
    isLoading.set(true);
    error.set(null);

    let currentCursor = 0;
    if (!reset) {
      const unsub = scanCursor.subscribe((v) => (currentCursor = v));
      unsub();
    }

    if (pattern !== undefined) {
      searchPattern.set(pattern);
    }

    let currentPattern = '*';
    const unsub2 = searchPattern.subscribe((v) => (currentPattern = v));
    unsub2();

    const result = await invoke<ScanResult>('scan_keys', {
      pattern: currentPattern,
      cursor: currentCursor,
      count: 200,
    });

    if (reset) {
      keys.set(result.keys);
    } else {
      keys.update((existing) => {
        const existingNames = new Set(existing.map((k) => k.name));
        const newKeys = result.keys.filter((k) => !existingNames.has(k.name));
        return [...existing, ...newKeys];
      });
    }

    scanCursor.set(result.cursor);
    hasMore.set(result.cursor !== 0);
  } catch (e: any) {
    error.set(e.toString());
  } finally {
    isLoading.set(false);
  }
}

export async function loadKeyDetail(key: string) {
  try {
    isLoadingDetail.set(true);
    error.set(null);
    selectedKey.set(key);
    const detail = await invoke<KeyDetail>('get_key_detail', { key });
    keyDetail.set(detail);
  } catch (e: any) {
    error.set(e.toString());
  } finally {
    isLoadingDetail.set(false);
  }
}

export async function deleteSelectedKeys(keysToDelete: string[]) {
  try {
    error.set(null);
    await invoke<number>('delete_keys', { keys: keysToDelete });
    keys.update((list) => list.filter((k) => !keysToDelete.includes(k.name)));
    selectedKeys.set(new Set());

    let currentSelected: string | null = null;
    const unsub = selectedKey.subscribe((v) => (currentSelected = v));
    unsub();

    if (currentSelected && keysToDelete.includes(currentSelected)) {
      selectedKey.set(null);
      keyDetail.set(null);
    }
  } catch (e: any) {
    error.set(e.toString());
  }
}

export async function updateKeyValue(key: string, value: string, ttl?: number) {
  try {
    error.set(null);
    await invoke('set_key_value', { key, value, ttl: ttl ?? null });
    await loadKeyDetail(key);
  } catch (e: any) {
    error.set(e.toString());
  }
}

export async function updateHashField(key: string, field: string, value: string) {
  try {
    error.set(null);
    await invoke('set_hash_field', { key, field, value });
    await loadKeyDetail(key);
  } catch (e: any) {
    error.set(e.toString());
  }
}

export async function removeHashField(key: string, field: string) {
  try {
    error.set(null);
    await invoke('delete_hash_field', { key, field });
    await loadKeyDetail(key);
  } catch (e: any) {
    error.set(e.toString());
  }
}

export async function updateKeyTtl(key: string, ttl: number) {
  try {
    error.set(null);
    await invoke('set_key_ttl', { key, ttl });
    await loadKeyDetail(key);
  } catch (e: any) {
    error.set(e.toString());
  }
}

export async function renameRedisKey(oldKey: string, newKey: string) {
  try {
    error.set(null);
    await invoke('rename_key', { oldKey, newKey });
    keys.update((list) =>
      list.map((k) => (k.name === oldKey ? { ...k, name: newKey } : k))
    );
    selectedKey.set(newKey);
    await loadKeyDetail(newKey);
  } catch (e: any) {
    error.set(e.toString());
  }
}

export async function addListItem(key: string, value: string) {
  try {
    error.set(null);
    await invoke('add_list_item', { key, value });
    await loadKeyDetail(key);
  } catch (e: any) {
    error.set(e.toString());
  }
}

export async function addSetMember(key: string, value: string) {
  try {
    error.set(null);
    await invoke('add_set_member', { key, value });
    await loadKeyDetail(key);
  } catch (e: any) {
    error.set(e.toString());
  }
}
