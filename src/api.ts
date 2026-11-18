import { invoke, isTauri } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import type { Snapshot } from './types';
export const browserPreview = !isTauri();
const empty: Snapshot = {
  game: null,
  mods: [],
  backups: [],
  recovery: [],
  data_path: 'Available in the desktop app',
  storage_bytes: 0,
  archive_bytes: 0,
  backup_bytes: 0,
};
export async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (browserPreview) {
    if (command === 'snapshot') return empty as T;
    if (command === 'detect_games') return [] as T;
    if (command === 'organization_snapshot')
      return { collections: [], suits: [], available_members: [], warnings: [] } as T;
    throw new Error(
      'Open the Zoxryn Mods desktop app to manage local files. Browser preview cannot modify your game.',
    );
  }
  return invoke<T>(command, args);
}
export async function pick(kind: 'game' | 'archive' | 'preview'): Promise<string | null> {
  if (browserPreview) throw new Error('File selection is available in the Windows desktop app.');
  const result = await open(
    kind === 'game'
      ? { directory: true, multiple: false, title: 'Select your GTA V Legacy folder' }
      : kind === 'preview'
        ? {
            multiple: false,
            title: 'Assign local preview artwork',
            filters: [{ name: 'Raster artwork', extensions: ['png', 'jpg', 'jpeg', 'webp'] }],
          }
        : {
            multiple: false,
            title: 'Import a mod archive',
            filters: [{ name: 'Mod archives', extensions: ['zip', '7z', 'rar', 'oiv'] }],
          },
  );
  return typeof result === 'string' ? result : null;
}
