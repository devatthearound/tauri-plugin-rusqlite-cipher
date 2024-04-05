import { invoke } from '@tauri-apps/api/tauri'
export default class Rusqlite {
  name: string;
  useSqlCipher: boolean;
  encryptionKey?: string;

  constructor(name: string, useSqlCipher: boolean = false, encryptionKey?: string) {
    this.name = name;
    this.useSqlCipher = useSqlCipher;
    this.encryptionKey = encryptionKey;
  }

  static async openInMemory(name: string, useSqlCipher: boolean = false, encryptionKey?: string): Promise<Rusqlite> {
    return await invoke('plugin:rusqlite|open_in_memory', { name, useSqlCipher, encryptionKey })
      .then(() => new Rusqlite(name, useSqlCipher, encryptionKey));
  }

  static async openInPath(path: string, useSqlCipher: boolean = false, encryptionKey?: string): Promise<Rusqlite> {
    return await invoke('plugin:rusqlite|open_in_path', { path, useSqlCipher, encryptionKey })
      .then(() => new Rusqlite(path, useSqlCipher, encryptionKey));
  }


  async migration(migrations: Migration[]): Promise<void> {
    return await invoke('plugin:rusqlite|migration', {name: this.name, migrations});
  }
  
  async update(sql: string, parameters: Map<string, any>): Promise<void> {
    return await invoke('plugin:rusqlite|update', {name: this.name, sql, parameters});
  }

  async select(sql: string, parameters:Map<string, any>): Promise<any[]> {
    return await invoke('plugin:rusqlite|select', {name: this.name, sql, parameters});
  }

  async batch(batchSql: string): Promise<void> {
    return await invoke('plugin:rusqlite|batch', {name: this.name, batch_sql: batchSql});
  }

  async close(): Promise<void> {
    return await invoke('plugin:rusqlite|close', {name: this.name});
  }
}

export interface Migration {
  name: string;
  sql: string;
}
