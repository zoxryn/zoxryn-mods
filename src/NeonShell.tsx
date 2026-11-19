import {
  Minus,
  Square,
  X,
  Bell,
  Gamepad2,
  Search,
  Layers3,
  ShieldCheck,
  Database,
  Boxes,
  Shirt,
  Palette,
  Gauge,
} from 'lucide-react';
import { useEffect, useState } from 'react';
import { browserPreview } from './api';
import type { OrganizationSnapshot } from './organizationTypes';

type Appearance = { glow: 'quiet' | 'balanced' | 'vivid'; motion: 'system' | 'reduced' };
export function useAppearance() {
  const [appearance, setAppearance] = useState<Appearance>(() => {
    try {
      const value = JSON.parse(localStorage.getItem('zoxryn.appearance.v1') ?? '{}');
      return {
        glow: ['quiet', 'balanced', 'vivid'].includes(value.glow) ? value.glow : 'balanced',
        motion: value.motion === 'reduced' ? 'reduced' : 'system',
      };
    } catch {
      return { glow: 'balanced', motion: 'system' };
    }
  });
  useEffect(() => {
    document.documentElement.dataset.glow = appearance.glow;
    document.documentElement.dataset.motion = appearance.motion;
    try {
      localStorage.setItem('zoxryn.appearance.v1', JSON.stringify(appearance));
    } catch {
      /* Preferences remain usable when storage is unavailable. */
    }
  }, [appearance]);
  return { appearance, setAppearance };
}
export function AppearanceSettings({
  appearance,
  setAppearance,
}: ReturnType<typeof useAppearance>) {
  return (
    <section className="neon-settings-section" aria-label="Appearance settings">
      <div className="settings-section-heading">
        <Palette size={22} />
        <div>
          <span className="eyebrow">MAKE IT YOURS</span>
          <h2>Appearance</h2>
        </div>
      </div>
      <div className="appearance-preview" aria-hidden="true">
        <span className="preview-monogram">Z</span>
        <span>
          NEON COLLECTION<small>Original design. Your workspace.</small>
        </span>
      </div>
      <label className="settings-control">
        <span>
          <strong>Glow strength</strong>
          <small>Static accents around selected cards and controls.</small>
        </span>
        <select
          aria-label="Glow strength"
          value={appearance.glow}
          onChange={(e) =>
            setAppearance({ ...appearance, glow: e.target.value as Appearance['glow'] })
          }
        >
          <option value="quiet">Quiet</option>
          <option value="balanced">Balanced</option>
          <option value="vivid">Vivid</option>
        </select>
      </label>
      <label className="settings-control">
        <span>
          <strong>Reduced motion</strong>
          <small>Windows reduced-motion preferences always take priority.</small>
        </span>
        <select
          aria-label="Reduced motion"
          value={appearance.motion}
          onChange={(e) =>
            setAppearance({ ...appearance, motion: e.target.value as Appearance['motion'] })
          }
        >
          <option value="system">Follow Windows</option>
          <option value="reduced">Always reduce</option>
        </select>
      </label>
    </section>
  );
}
export function NeonTitleBar({
  connected,
  update,
  onUpdates,
}: {
  connected: boolean;
  update: string | null;
  onUpdates: () => void;
}) {
  async function control(action: 'minimize' | 'toggleMaximize' | 'close') {
    if (browserPreview) return;
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await getCurrentWindow()[action]();
  }
  return (
    <header className="neon-titlebar" data-tauri-drag-region>
      <div className="neon-wordmark" data-tauri-drag-region>
        <span className="neon-z">Z</span>
        <span>
          ZOXRYN<small>MODS · NEON COLLECTION</small>
        </span>
      </div>
      <div className="neon-panorama" data-tauri-drag-region aria-hidden="true" />
      <div className="window-controls">
        <button
          aria-label="Minimize window"
          disabled={browserPreview}
          onClick={() => void control('minimize')}
        >
          <Minus size={17} />
        </button>
        <button
          aria-label="Maximize or restore window"
          disabled={browserPreview}
          onClick={() => void control('toggleMaximize')}
        >
          <Square size={14} />
        </button>
        <button
          aria-label="Close window"
          disabled={browserPreview}
          onClick={() => void control('close')}
        >
          <X size={19} />
        </button>
      </div>
      <div className="titlebar-status">
        <span className={connected ? 'connection-chip connected' : 'connection-chip'}>
          <Gamepad2 size={14} />
          {connected ? 'GTA V Legacy connected' : 'GTA V not connected'}
        </span>
        <button
          className={update ? 'update-indicator available' : 'update-indicator'}
          aria-label={update ? `Application update available ${update}` : 'Application updates'}
          onClick={onUpdates}
        >
          <Bell size={19} />
          {update && <i />}
        </button>
        <span className="neon-profile" aria-label="Local workspace">
          Z
        </span>
      </div>
    </header>
  );
}
export function GlobalSearch({
  data,
  onSuit,
  onCollection,
  onMod,
}: {
  data: OrganizationSnapshot;
  onSuit: (id: string) => void;
  onCollection: (id: string) => void;
  onMod: (id: string) => void;
}) {
  const [query, setQuery] = useState('');
  const matches = query.trim()
    ? [
        ...data.collections.map((c) => ({
          id: c.id,
          name: c.name,
          type: 'Collection',
          go: () => onCollection(c.id),
        })),
        ...data.suits.map((s) => ({
          id: s.id,
          name: s.name,
          type: 'Suit / resource',
          go: () => onSuit(s.id),
        })),
        ...data.available_members
          .filter((m) => m.reference.kind === 'managed')
          .map((m) => ({
            id: m.reference.id,
            name: m.name,
            type: 'Managed mod',
            go: () => onMod(m.reference.id),
          })),
      ]
        .filter((c) => c.name.toLowerCase().includes(query.trim().toLowerCase()))
        .slice(0, 8)
    : [];
  return (
    <div className="global-search">
      <Search size={17} />
      <input
        aria-label="Global search"
        placeholder="Search your collections & suits…"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Escape') setQuery('');
        }}
      />
      {query && (
        <div className="global-results" role="region" aria-label="Global search results">
          {matches.length ? (
            matches.map((m) => (
              <button
                key={m.type + m.id}
                onClick={() => {
                  m.go();
                  setQuery('');
                }}
              >
                <Layers3 size={16} />
                <span>
                  {m.name}
                  <small>{m.type}</small>
                </span>
              </button>
            ))
          ) : (
            <p>No matching collections or suits</p>
          )}
        </div>
      )}
    </div>
  );
}
export function CollectionMetrics({
  data,
  backups,
  backupBytes,
  onNavigate,
}: {
  data: OrganizationSnapshot;
  backups: number;
  backupBytes: string;
  onNavigate: (view: 'library' | 'suits' | 'backups') => void;
}) {
  return (
    <div className="collection-metrics">
      <button onClick={() => onNavigate('library')}>
        <Boxes size={22} />
        <span>
          <strong>{data.collections.length}</strong> Collections
        </span>
      </button>
      <button onClick={() => onNavigate('suits')}>
        <Shirt size={22} />
        <span>
          <strong>{data.suits.length}</strong> Identified outfits
        </span>
      </button>
      <button onClick={() => onNavigate('backups')}>
        <ShieldCheck size={22} />
        <span>
          <strong>{backups}</strong> Recorded backups<small>{backupBytes} retained</small>
        </span>
        <Database size={17} />
      </button>
    </div>
  );
}
export function PerformanceInfo() {
  return (
    <section className="neon-settings-section">
      <div className="settings-section-heading">
        <Gauge size={22} />
        <h2>Performance</h2>
      </div>
      <p>
        Static skyline and glow. No animated wallpaper, particle system or background rendering
        loop.
      </p>
      <div className="settings-facts">
        <span>24 cards per page</span>
        <span>16 cached thumbnails</span>
        <span>3 image reads at a time</span>
        <span>On-demand scanning</span>
      </div>
    </section>
  );
}
