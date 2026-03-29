export interface ConnectionInfo {
  id: string;
  name: string;
  mode: string;
  url: string;
}

export interface ConnectionTab {
  id: string;
  name: string;
  mode: string;
  is_active: boolean;
}

export interface ServerInfo {
  version: string;
  mode: string;
  connected_clients: string;
  used_memory_human: string;
  total_keys: string;
  uptime_in_seconds: string;
}

export interface KeyEntry {
  name: string;
  key_type: string;
}

export interface ScanResult {
  cursor: number;
  keys: KeyEntry[];
}

export interface HashField {
  field: string;
  value: string;
}

export interface ZSetMember {
  member: string;
  score: number;
}

export type KeyValueData =
  | { type: 'String'; data: string }
  | { type: 'Hash'; data: HashField[] }
  | { type: 'List'; data: string[] }
  | { type: 'Set'; data: string[] }
  | { type: 'ZSet'; data: ZSetMember[] }
  | { type: 'Unknown'; data: string };

export interface KeyDetail {
  key: string;
  key_type: string;
  ttl: number;
  value: KeyValueData;
  size: number;
}

export interface SavedConnection {
  id: string;
  name: string;
  url: string;
  password: string;
}
