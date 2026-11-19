import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { version as packageVersion } from '../package.json';
import {
  Activity,
  Archive,
  ArrowDownToLine,
  ArrowRight,
  ArrowUpRight,
  Bell,
  Box,
  Shirt,
  Car,
  Check,
  CheckCheck,
  ChevronRight,
  CircleHelp,
  Clock3,
  Database,
  Download,
  ExternalLink,
  FileArchive,
  FileCheck2,
  Folder,
  FolderOpen,
  Gamepad2,
  HardDrive,
  LayoutDashboard,
  Library,
  LoaderCircle,
  Map,
  MoreHorizontal,
  PackageCheck,
  Plus,
  Power,
  RefreshCw,
  Search,
  Settings2,
  Shield,
  ShieldCheck,
  SlidersHorizontal,
  Sparkles,
  Trash2,
  TriangleAlert,
  Undo2,
  Upload,
  X,
  Zap,
  FileSearch,
} from 'lucide-react';
import { browserPreview, call, pick } from './api';
import { providers } from './providers';
import { useAppUpdates } from './useAppUpdates';
import { UpdateSettings, UpdatePrompt } from './AppUpdates';
import InstalledMods, { inventoryCounts } from './InstalledMods';
import OrganizationPage from './OrganizationPage';
import { emptyOrganization, type OrganizationSnapshot } from './organizationTypes';
import {
  AppearanceSettings,
  CollectionMetrics,
  GlobalSearch,
  NeonTitleBar,
  PerformanceInfo,
  useAppearance,
} from './NeonShell';
import FileGroups from './FileGroups';
import Thumbnail from './Thumbnail';
import UnidentifiedFiles from './UnidentifiedFiles';
import ModHealth from './ModHealth';
import type { Backup, Game, Mod, Plan, Report, Snapshot, ScanInventory } from './types';

type View =
  | 'dashboard'
  | 'installed'
  | 'library'
  | 'suits'
  | 'unidentified'
  | 'game'
  | 'backups'
  | 'discover'
  | 'settings';
const nav = [
  { id: 'dashboard', label: 'Overview', icon: LayoutDashboard },
  { id: 'installed', label: 'Installed Mods', icon: PackageCheck },
  { id: 'library', label: 'My Mods', icon: Library },
  { id: 'suits', label: 'Suits & Skins', icon: Shirt },
  { id: 'unidentified', label: 'Unidentified Files', icon: FileSearch },
  { id: 'discover', label: 'Discover mods', icon: Sparkles },
  { id: 'game', label: 'Game & dependencies', icon: Gamepad2 },
  { id: 'backups', label: 'Backups & recovery', icon: Archive },
] as const;
const categories = [
  'All categories',
  'Vehicles',
  'Police',
  'Superheroes',
  'Graphics',
  'Scripts',
  'Maps',
  'Characters',
  'Gameplay',
];
const bytes = (n: number) =>
  n >= 1073741824
    ? `${(n / 1073741824).toFixed(1)} GB`
    : n >= 1048576
      ? `${(n / 1048576).toFixed(1)} MB`
      : n >= 1024
        ? `${(n / 1024).toFixed(1)} KB`
        : `${n} B`;
const date = (n: number) =>
  new Date(n * 1000).toLocaleDateString(undefined, {
    month: 'short',
    day: 'numeric',
    year: 'numeric',
  });
const categoryIcon = (category: string) =>
  category === 'Vehicles'
    ? Car
    : category === 'Maps'
      ? Map
      : category === 'Scripts'
        ? Zap
        : category === 'Police'
          ? Shield
          : Box;
function Logo() {
  return (
    <span className="logo" aria-hidden="true">
      <svg viewBox="0 0 40 40">
        <path d="M10 11h23L16 27h15l-3 5H6L24 16H8z" fill="currentColor" />
        <path d="m27 5 7 2-2 7-4-3z" fill="#d4b6ff" />
      </svg>
    </span>
  );
}
function Status({ status }: { status: string }) {
  return (
    <span className={`badge ${status}`}>
      <i />
      {status === 'imported' ? 'Ready to review' : status[0].toUpperCase() + status.slice(1)}
    </span>
  );
}
function Empty({
  title,
  text,
  action,
  actionLabel = 'Import your first mod',
}: {
  title: string;
  text: string;
  action?: () => void;
  actionLabel?: string;
}) {
  return (
    <div className="empty">
      <div className="empty-icon">
        <Box size={28} />
      </div>
      <h3>{title}</h3>
      <p>{text}</p>
      {action && (
        <button className="button primary" onClick={action}>
          <Plus size={16} />
          {actionLabel}
        </button>
      )}
    </div>
  );
}
function City() {
  return (
    <div className="city" aria-hidden="true">
      <div className="sun" />
      <svg viewBox="0 0 620 240" preserveAspectRatio="xMidYMax slice">
        <defs>
          <linearGradient id="cityg" x2="0" y2="1">
            <stop stopColor="#4b326c" />
            <stop offset="1" stopColor="#20182f" />
          </linearGradient>
        </defs>
        <path
          d="M0 210h25v-47h30v23h24v-71h35v-10h12v-22h6v22h12v10h19v85h18V60h12V46h8V30h3v16h8v14h12v-7h23v147h15V111h29v-9h27v98h18V91h15V77h25v14h15v109h17V42h11V31h17v11h12v158h15v-77h38v77h16v-93h32v93h12v-58h33v58h22V83h31v117h23v-46h35v86H0z"
          fill="url(#cityg)"
        />
        <path
          d="M0 228h72v-20h43v20h46v-44h27v44h32v-22h41v22h60v-33h40v33h44v-17h48v17h58v-36h26v36h63v12H0z"
          fill="#191420"
        />
        {[197, 221, 380, 397, 291, 452, 517].map((x) => (
          <path
            key={x}
            d={`M${x} 132v37m0 8v12`}
            stroke="#b195d2"
            opacity=".27"
            strokeWidth="3"
            strokeDasharray="3 6"
          />
        ))}
      </svg>
      <span>LOS SANTOS, YOUR WAY.</span>
    </div>
  );
}

export default function App() {
  const updates = useAppUpdates();
  const appearance = useAppearance();
  const [data, setData] = useState<Snapshot>({
    game: null,
    mods: [],
    backups: [],
    recovery: [],
    data_path: '',
    storage_bytes: 0,
    archive_bytes: 0,
    backup_bytes: 0,
  });
  const [view, setView] = useState<View>('dashboard');
  const [libraryMode, setLibraryMode] = useState<'collections' | 'individual'>('collections');
  const [organizationSelection, setOrganizationSelection] = useState<string | null>(null);
  const [advancedInspector, setAdvancedInspector] = useState(false);
  const [organization, setOrganization] = useState<OrganizationSnapshot>(emptyOrganization);
  const [organizationLoading, setOrganizationLoading] = useState(false);
  const [organizationError, setOrganizationError] = useState<string | null>(null);
  const organizationRequest = useRef(0);
  const refreshOrganization = useCallback(async () => {
    const request = ++organizationRequest.current;
    setOrganizationLoading(true);
    try {
      const value = await call<OrganizationSnapshot>('organization_snapshot');
      if (request === organizationRequest.current) {
        setOrganization(value);
        setOrganizationError(null);
      }
    } catch (error) {
      if (request === organizationRequest.current) setOrganizationError(String(error));
    } finally {
      if (request === organizationRequest.current) setOrganizationLoading(false);
    }
  }, []);
  useEffect(() => {
    void refreshOrganization();
  }, [view, data.mods, data.inventory, refreshOrganization]);
  const [query, setQuery] = useState('');
  const [category, setCategory] = useState('All categories');
  const [filter, setFilter] = useState('all');
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<{ text: string; error: boolean } | null>(null);
  const [selected, setSelected] = useState<Mod | null>(null);
  const [plan, setPlan] = useState<Plan | null>(null);
  const [approved, setApproved] = useState(false);
  const [reviewed, setReviewed] = useState(false);
  const [ownershipReviewed, setOwnershipReviewed] = useState(false);
  const [scanning, setScanning] = useState(false);
  const scanFlight = useRef(false);
  const [games, setGames] = useState<Game[] | null>(null);
  const [safety, setSafety] = useState(false);
  const [report, setReport] = useState<Report | null>(null);
  const notify = useCallback((text: string, error = false) => setNotice({ text, error }), []);
  const refresh = useCallback(async () => {
    const s = await call<Snapshot>('snapshot');
    setData(s);
    return s;
  }, []);
  const autoDetected = useRef(false);
  const rescan = useCallback(async () => {
    if (scanFlight.current || browserPreview) return;
    scanFlight.current = true;
    setScanning(true);
    try {
      const inventory = await call<ScanInventory>('scan_installed');
      setData((previous) =>
        previous.game?.path === inventory.game_path ? { ...previous, inventory } : previous,
      );
    } catch (e) {
      notify(String(e), true);
    } finally {
      scanFlight.current = false;
      setScanning(false);
    }
  }, [notify]);
  useEffect(() => {
    void refresh()
      .then(async (saved) => {
        if (saved.game?.edition === 'Legacy' && !browserPreview) void rescan();
        if (!saved.game && !autoDetected.current && !browserPreview) {
          autoDetected.current = true;
          const found = await call<Game[]>('detect_games');
          if (found.length) setGames(found);
        }
      })
      .catch((e) => notify(String(e), true));
  }, [refresh, notify, rescan]);
  useEffect(() => {
    if (!notice || notice.error) return;
    const t = setTimeout(() => setNotice(null), 9000);
    return () => clearTimeout(t);
  }, [notice]);
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && !busy) {
        setPlan(null);
        setSelected(null);
        setGames(null);
        setSafety(false);
        setReport(null);
      }
    };
    window.addEventListener('keydown', key);
    return () => window.removeEventListener('keydown', key);
  }, [busy]);
  useEffect(() => {
    if (!selected && !plan && !games && !safety && !report) return;
    const previous = document.activeElement as HTMLElement | null;
    const dialog = document.querySelector<HTMLElement>('[role="dialog"]');
    if (!dialog) return;
    const focusable = () => [
      ...dialog.querySelectorAll<HTMLElement>(
        'button:not(:disabled), input:not(:disabled), select:not(:disabled), a[href], summary',
      ),
    ];
    focusable()[0]?.focus();
    const trap = (e: KeyboardEvent) => {
      if (e.key !== 'Tab') return;
      const elements = focusable();
      const first = elements[0];
      const last = elements.at(-1);
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last?.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first?.focus();
      }
    };
    dialog.addEventListener('keydown', trap);
    return () => {
      dialog.removeEventListener('keydown', trap);
      if (previous?.isConnected) previous.focus();
    };
  }, [selected, plan, games, safety, report]);
  async function task(fn: () => Promise<void>) {
    if (busy || updates.active) return;
    setBusy(true);
    try {
      await fn();
    } catch (e) {
      notify(String(e), true);
    } finally {
      setBusy(false);
    }
  }
  async function importMod() {
    await task(async () => {
      const path = await pick('archive');
      if (!path) return;
      const mod = await call<Mod>('import_archive', { path });
      await refresh();
      setSelected(mod);
      notify(`Inspected ${mod.name}. Review its files and instructions before installing.`);
    });
  }
  async function chooseGame() {
    await task(async () => {
      const path = await pick('game');
      if (!path) return;
      await call('select_game', { path, launcher: 'Manual selection' });
      await refresh();
      setGames(null);
      notify('Game folder saved and installed-mod inventory scanned. No game files were changed.');
    });
  }
  async function detectGames() {
    await task(async () => {
      const found = await call<Game[]>('detect_games');
      setGames(found);
    });
  }
  async function operation(mod: Mod, operation: string) {
    await task(async () => {
      const next = await call<Plan>('prepare_operation', { id: mod.id, operation });
      setPlan(next);
      setApproved(false);
      setReviewed(false);
      setSelected(null);
    });
  }
  async function suitOperation(id: string, operation: string) {
    await task(async () => {
      const next = await call<Plan>('prepare_suit', { id, operation });
      setPlan(next);
      setApproved(false);
      setReviewed(false);
      setOwnershipReviewed(false);
      setSelected(null);
    });
  }
  async function restore(backup: Backup) {
    await task(async () => {
      const next = await call<Plan>('prepare_restore', { id: backup.id });
      setPlan(next);
      setApproved(false);
      setReviewed(false);
    });
  }
  async function clean() {
    await task(async () => {
      const next = await call<Plan>('prepare_clean');
      setPlan(next);
      setApproved(false);
      setReviewed(false);
    });
  }
  async function adopt(id: string) {
    await task(async () => {
      const next = await call<Plan>('prepare_adoption', { id });
      setPlan(next);
      setApproved(false);
      setReviewed(false);
      setOwnershipReviewed(false);
      setSelected(null);
    });
  }
  async function apply() {
    if (!plan || updates.active) return;
    await task(async () => {
      const r = await call<Report>('apply_plan', {
        token: plan.token,
        approved,
        compatibilityReviewed: reviewed,
        ...(plan.operation === 'adopt' || plan.independent_scope ? { ownershipReviewed } : {}),
      });
      setPlan(null);
      await refresh();
      setReport(r);
      if (data.game?.edition === 'Legacy') void rescan();
    });
  }
  const game = data.game;
  const healthCheck = () =>
    void task(async () => {
      await call('mod_health_check');
      await refresh();
      await refreshOrganization();
    });
  const planIsRelease =
    plan?.operation === 'remove' && data.mods.some((m) => m.id === plan.mod_id && !!m.adopted_from);
  const detected = inventoryCounts(data.inventory, data.mods);
  const installed = data.mods.filter((m) => m.status !== 'imported');
  const enabled = installed.filter((m) => m.status === 'enabled');
  const missingIds = new Set(
    enabled
      .flatMap((m) => m.requirements)
      .filter((r) => !game?.prerequisites.some((p) => p.id === r && p.present)),
  );
  const size = installed
    .filter((m) => m.status === 'enabled')
    .reduce((n, m) => n + m.files.reduce((s, f) => s + f.size, 0), 0);
  const filtered = useMemo(
    () =>
      data.mods.filter(
        (m) =>
          (!m.adopted_from || m.status !== 'imported') &&
          (category === 'All categories' || m.category === category) &&
          (filter === 'all' || m.status === filter) &&
          `${m.name} ${m.author} ${m.category}`.toLowerCase().includes(query.toLowerCase()),
      ),
    [data.mods, category, filter, query],
  );
  const currentName = nav.find((n) => n.id === view)?.label ?? 'Settings';
  const iconButton = (
    label: string,
    icon: React.ReactNode,
    onClick: () => void,
    disabled = false,
  ) => (
    <button
      className="icon-button"
      title={label}
      aria-label={label}
      disabled={disabled || busy}
      onClick={onClick}
    >
      {icon}
    </button>
  );
  function ModTable({ limit }: { limit?: number }) {
    const rows = limit ? filtered.slice(0, limit) : filtered;
    return rows.length ? (
      <div className="table-wrap">
        <table className="mods-table">
          <thead>
            <tr>
              <th>MOD NAME</th>
              <th>CATEGORY</th>
              <th>STATUS</th>
              <th>SIZE</th>
              <th aria-label="Actions" />
            </tr>
          </thead>
          <tbody>
            {rows.map((mod) => {
              const Icon = categoryIcon(mod.category);
              return (
                <tr key={mod.id}>
                  <td>
                    <button className="mod-link" onClick={() => setSelected(mod)}>
                      <span className={`mod-icon ${mod.category.toLowerCase()}`}>
                        <Icon size={20} />
                      </span>
                      <span>
                        <strong>{mod.name}</strong>
                        <small>
                          {mod.author} <b>·</b>{' '}
                          {mod.version === 'Unknown' ? 'Version unverified' : `v${mod.version}`}
                        </small>
                      </span>
                    </button>
                  </td>
                  <td>
                    <span className="category-label">{mod.category}</span>
                  </td>
                  <td>
                    <Status status={mod.status} />
                  </td>
                  <td className="muted mono">{bytes(mod.files.reduce((n, f) => n + f.size, 0))}</td>
                  <td>
                    <div className="row-actions">
                      {mod.status === 'imported'
                        ? iconButton(
                            `Review installation of ${mod.name}`,
                            <ArrowDownToLine size={16} />,
                            () => void operation(mod, 'install'),
                            !game || !mod.files.length,
                          )
                        : iconButton(
                            `${mod.status === 'enabled' ? 'Disable' : 'Enable'} ${mod.name}`,
                            <Power size={16} />,
                            () =>
                              void operation(mod, mod.status === 'enabled' ? 'disable' : 'enable'),
                            !game,
                          )}
                      {iconButton(`Details for ${mod.name}`, <MoreHorizontal size={18} />, () =>
                        setSelected(mod),
                      )}
                    </div>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    ) : (
      <Empty
        title={
          query || category !== 'All categories' || filter !== 'all'
            ? 'No matching mods'
            : 'Your next adventure starts here'
        }
        text={
          data.mods.length
            ? 'Try another search or filter.'
            : 'Import a downloaded archive. We’ll inspect it, check dependencies, and show you exactly what will change.'
        }
        action={data.mods.length ? undefined : () => void importMod()}
      />
    );
  }
  function GameCard() {
    return (
      <section className="panel game-card">
        <div className="section-heading">
          <h2>Your game</h2>
          <Gamepad2 size={18} />
        </div>
        <div className="game-cover">
          <div className="cover-lines" />
          <span>
            GRAND
            <br />
            THEFT
            <br />
            AUTO <b>V</b>
          </span>
          <small>{game?.edition === 'Enhanced' ? 'ENHANCED' : 'LEGACY'}</small>
        </div>
        <div className="game-title">
          <h3>Grand Theft Auto V</h3>
          <span className={`dot ${game ? 'green' : ''}`} />
        </div>
        <p className="subtle">
          {game ? `${game.launcher} · ${game.edition}` : 'Connect your Legacy installation'}
        </p>
        <div className="game-meta">
          <span>Game build</span>
          <strong>{game?.version ?? 'Not verified'}</strong>
        </div>
        <div className="game-meta">
          <span>Installation</span>
          <strong className={game ? 'green-text' : ''}>
            {game ? 'Detected' : 'Not connected'}
          </strong>
        </div>
        {game ? (
          <button
            className="button full"
            disabled={busy}
            onClick={() =>
              void task(async () => {
                await call('open_location', { kind: 'game' });
              })
            }
          >
            <FolderOpen size={15} />
            Open GTA folder
            <ArrowUpRight size={14} />
          </button>
        ) : (
          <button className="button full" onClick={() => void chooseGame()} disabled={busy}>
            <Folder size={15} />
            Select game folder
            <ArrowRight size={14} />
          </button>
        )}
      </section>
    );
  }
  return (
    <div className="app-shell" data-view={view}>
      <NeonTitleBar
        connected={game?.edition === 'Legacy'}
        update={updates.status.available_version}
        onUpdates={() => {
          setView('settings');
          if (updates.status.available_version) updates.setPrompt(true);
        }}
      />
      <UpdatePrompt updates={updates} fileBusy={busy || scanning} />
      <aside className="sidebar">
        <a
          className="brand"
          href="#"
          onClick={(e) => {
            e.preventDefault();
            setView('dashboard');
          }}
        >
          <Logo />
          <span>
            ZOXRYN <b>MODS</b>
            <small>DISCOVER. INSTALL. PLAY.</small>
          </span>
        </a>
        <div className="workspace-label">WORKSPACE</div>
        <nav aria-label="Main navigation">
          {nav.map((item) => (
            <button
              key={item.id}
              aria-label={item.label}
              title={item.label}
              aria-current={view === item.id ? 'page' : undefined}
              className={view === item.id ? 'nav-item active' : 'nav-item'}
              onClick={() => setView(item.id)}
            >
              <item.icon size={18} />
              <span>{item.label}</span>
              {item.id === 'library' && <em>{data.mods.length}</em>}
              {item.id === 'discover' && <small>NEW</small>}
            </button>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <div className="local-card">
            <span className="local-shield">
              <ShieldCheck size={20} />
            </span>
            <strong>Your game. Your control.</strong>
            <p>
              Local files. Clear changes.
              <br />A backup before every install.
            </p>
            <button onClick={() => setSafety(true)}>
              Our safety approach
              <ArrowUpRight size={13} />
            </button>
          </div>
          <button
            className={`nav-item ${view === 'settings' ? 'active' : ''}`}
            aria-label="Settings"
            title="Settings"
            onClick={() => setView('settings')}
          >
            <Settings2 size={18} />
            Settings
          </button>
          <button
            className="nav-item"
            aria-label="Help & getting started"
            title="Help & getting started"
            onClick={() => setSafety(true)}
          >
            <CircleHelp size={18} />
            Help & getting started
          </button>
          <div className="sidebar-footer">
            <span className="dot green" />
            LOCAL & PRIVATE <span>v{packageVersion}</span>
          </div>
        </div>
      </aside>
      <div className="main-shell">
        <header className="topbar">
          <div className="breadcrumb">
            <span className="neon-workspace-label">NEON COLLECTION</span>
            <ChevronRight size={13} />
            <strong>{currentName}</strong>
          </div>
          <GlobalSearch
            data={organization}
            onSuit={(id) => {
              setOrganizationSelection(id);
              setView('suits');
            }}
            onCollection={(id) => {
              setOrganizationSelection(id);
              setLibraryMode('collections');
              setView('library');
            }}
            onMod={(id) => setSelected(data.mods.find((m) => m.id === id) ?? null)}
          />
          <div className="topbar-right">
            {['library', 'installed', 'suits'].includes(view) && (
              <button className="button primary" disabled={busy} onClick={() => void importMod()}>
                <Plus size={15} />
                Import mod
              </button>
            )}
            {iconButton('Run Mod Health Check', <ShieldCheck size={18} />, healthCheck)}
            <span className="singleplayer">
              <ShieldCheck size={13} />
              Single-player only
            </span>
            <span className="vertical-divider" />
            {iconButton('Show safety information', <ShieldCheck size={17} />, () =>
              setSafety(true),
            )}
            <div className="avatar">Z</div>
          </div>
        </header>
        <main
          className={
            ['library', 'installed', 'suits'].includes(view) ? 'collection-main' : undefined
          }
        >
          {browserPreview && (
            <div className="preview-banner">
              <FileCheck2 size={15} />
              Browser preview · Open the Windows desktop app to manage local files.
            </div>
          )}
          <div className="page-heading">
            <div>
              <div className="eyebrow">
                {view === 'dashboard' ? 'MAKE LOS SANTOS YOURS' : 'ZOXRYN WORKSPACE'}
              </div>
              <h1>{view === 'dashboard' ? 'Your modding command center.' : currentName}</h1>
              <p>
                {view === 'dashboard'
                  ? 'Less setup. More Los Santos. Everything you need, in one place.'
                  : view === 'installed'
                    ? 'Discover existing ASI, script, RAGE, and OpenIV files without changing your game.'
                    : view === 'library'
                      ? 'Every archive, dependency, and tracked file. All under your control.'
                      : view === 'suits'
                        ? 'Inspect character resources, organize outfits and review supported installation changes.'
                        : view === 'backups'
                          ? 'A clear history of every change, with a way back.'
                          : view === 'discover'
                            ? 'Find your next favorite. Download from the creator, then import it here.'
                            : view === 'game'
                              ? 'Start with the right edition and the right tools.'
                              : 'A lightweight workspace that keeps your records on this PC.'}
              </p>
            </div>
            {!['library', 'installed', 'suits'].includes(view) && (
              <button className="button primary" onClick={() => void importMod()} disabled={busy}>
                {busy ? <LoaderCircle className="spin" size={16} /> : <Plus size={17} />}Import mod
              </button>
            )}
          </div>
          {data.recovery.length > 0 && (
            <div className="alert danger">
              <TriangleAlert size={19} />
              <div>
                <strong>Recovery needs your attention</strong>
                {data.recovery.map((r, i) => (
                  <p key={i}>{r}</p>
                ))}
                <button
                  className="text-button"
                  disabled={busy}
                  onClick={() =>
                    void task(async () => {
                      await call('retry_recovery');
                      await refresh();
                    })
                  }
                >
                  Retry recovery
                </button>
              </div>
            </div>
          )}
          {view === 'dashboard' && (
            <>
              <CollectionMetrics
                data={organization}
                backups={data.backups.length}
                backupBytes={bytes(data.backup_bytes)}
                onNavigate={setView}
              />
              <ModHealth
                health={organization.health}
                busy={busy || scanning}
                onCheck={healthCheck}
              />
              <section className="hero">
                <div className="hero-copy">
                  <span className="pill">
                    <span className="dot purple" /> BUILT FOR GTA V LEGACY
                  </span>
                  <h2>
                    A new way to
                    <br />
                    make the game <em>yours.</em>
                  </h2>
                  <p>
                    Bring your favorite mods. We’ll handle the details.
                    <br />
                    Inspect first. Review every change. Always keep a way back.
                  </p>
                  <button
                    className="button hero-button"
                    onClick={() => (game ? setView('library') : void chooseGame())}
                    disabled={busy}
                  >
                    {game ? 'Explore your library' : 'Connect your game'}
                    <ArrowRight size={16} />
                  </button>
                </div>
                <City />
              </section>
              <div className="stats-grid inventory-stats">
                {[
                  {
                    icon: Box,
                    label: 'Total detected mods',
                    value: data.inventory ? detected.total : installed.length,
                    note: 'Recognized loading components',
                    color: 'purple',
                  },
                  {
                    icon: Power,
                    label: 'Enabled mods',
                    value: data.inventory ? detected.enabled : enabled.length,
                    note: 'Inferred locations / managed state',
                    color: 'green',
                  },
                  {
                    icon: Power,
                    label: 'Disabled mods',
                    value: data.inventory ? detected.disabled : installed.length - enabled.length,
                    note: 'Disabled suffixes / managed state',
                    color: 'blue',
                  },
                  {
                    icon: Archive,
                    label: 'OpenIV modifications',
                    value: detected.openiv,
                    note: 'Containers; changes unverified',
                    color: 'purple',
                  },
                  {
                    icon: TriangleAlert,
                    label: 'Mods requiring review',
                    value: detected.review,
                    note: 'Possible or unknown modifications',
                    color: 'amber',
                  },
                ].map((stat) => (
                  <div className="stat-card" key={stat.label}>
                    <div>
                      <span>{stat.label}</span>
                      <strong>{stat.value}</strong>
                      <small>{stat.note}</small>
                    </div>
                    <span className={`stat-icon ${stat.color}`}>
                      <stat.icon size={19} />
                    </span>
                  </div>
                ))}
              </div>
              <div className="inventory-dashboard-strip">
                <button className="text-button" onClick={() => setView('installed')}>
                  Open Installed Mods <ArrowRight size={14} />
                </button>
                <span>{detected.items} inventoried items</span>
                <span>{missingIds.size} missing dependencies for managed mods</span>
                <span>
                  {bytes(size + data.storage_bytes)} managed storage ·{' '}
                  {data.backups.filter((b) => b.status === 'committed').length} backups
                </span>
              </div>
              <div className="dashboard-grid">
                <section className="panel library-panel">
                  <div className="section-heading">
                    <h2>
                      Your mod library <span className="count">{data.mods.length}</span>
                    </h2>
                    <button className="text-button" onClick={() => setView('library')}>
                      View all
                      <ArrowRight size={14} />
                    </button>
                  </div>
                  <div className="table-toolbar">
                    <div className="tabs">
                      <button
                        className={filter === 'all' ? 'active' : ''}
                        onClick={() => setFilter('all')}
                      >
                        All mods
                      </button>
                      <button
                        className={filter === 'enabled' ? 'active' : ''}
                        onClick={() => setFilter('enabled')}
                      >
                        Enabled
                      </button>
                      <button
                        className={filter === 'disabled' ? 'active' : ''}
                        onClick={() => setFilter('disabled')}
                      >
                        Disabled
                      </button>
                    </div>
                    <Search size={17} className="muted" />
                  </div>
                  <ModTable limit={5} />
                </section>
                <GameCard />
              </div>
              <section className="panel recent-panel">
                <div className="section-heading">
                  <h2>
                    <Clock3 size={15} />
                    Recent installations & changes
                  </h2>
                  <span className="subtle">Recorded locally</span>
                </div>
                {data.backups.length ? (
                  data.backups.slice(0, 3).map((b) => (
                    <div className="recent-row" key={b.id}>
                      <span className="dot purple" />
                      <strong>{b.mod_name}</strong>
                      <span>
                        {b.operation} · {b.status.replace('_', ' ')}
                      </span>
                      <small>{date(b.created_at)}</small>
                    </div>
                  ))
                ) : (
                  <p className="recent-empty">
                    No changes yet. Your installation and recovery history will appear here.
                  </p>
                )}
              </section>
              <section className="safety-strip">
                <span className="safety-icon">
                  <ShieldCheck size={23} />
                </span>
                <div>
                  <strong>Mod with a little more peace of mind.</strong>
                  <p>Changes are tracked. Originals are backed up. You approve the plan.</p>
                </div>
                <button className="text-button" onClick={() => setView('backups')}>
                  View backups
                  <ArrowRight size={15} />
                </button>
              </section>
            </>
          )}
          {view === 'installed' && (
            <div className="org-library-tabs" role="tablist" aria-label="Installed mod views">
              <button
                className="button"
                role="tab"
                aria-selected={!advancedInspector}
                onClick={() => setAdvancedInspector(false)}
              >
                Organized packages
              </button>
              <button
                className="button"
                role="tab"
                aria-selected={advancedInspector}
                onClick={() => setAdvancedInspector(true)}
              >
                Advanced File Inspector
              </button>
              {!advancedInspector && (
                <button
                  className="button"
                  disabled={busy || scanning || !game}
                  onClick={() => void rescan()}
                >
                  <RefreshCw size={14} />
                  Rescan
                </button>
              )}
            </div>
          )}
          {view === 'installed' && advancedInspector && (
            <InstalledMods
              inventory={data.inventory}
              mods={data.mods}
              game={game}
              scanning={scanning}
              busy={busy || updates.active}
              onRescan={() => void rescan()}
              onAdopt={(id) => void adopt(id)}
              onOperation={(mod, op) => void operation(mod, op)}
            />
          )}
          {view === 'library' && (
            <div className="org-library-tabs" role="tablist" aria-label="Library organization">
              <button
                className="button"
                role="tab"
                aria-selected={libraryMode === 'collections'}
                onClick={() => setLibraryMode('collections')}
              >
                Collections
              </button>
              <button
                className="button"
                role="tab"
                aria-selected={libraryMode === 'individual'}
                onClick={() => setLibraryMode('individual')}
              >
                Individual mods
              </button>
            </div>
          )}
          {(view === 'suits' ||
            (view === 'installed' && !advancedInspector) ||
            (view === 'library' && libraryMode === 'collections')) && (
            <OrganizationPage
              key={view}
              mode={view === 'suits' ? 'suits' : 'collections'}
              focusId={organizationSelection}
              data={organization}
              loading={organizationLoading}
              error={organizationError}
              busy={busy || updates.active}
              onRefresh={refreshOrganization}
              onBusy={setBusy}
              onReviewSuit={(id, op) => void suitOperation(id, op)}
              onParent={(id) => setSelected(data.mods.find((m) => m.id === id) ?? null)}
            />
          )}
          {view === 'unidentified' && (
            <UnidentifiedFiles
              data={organization}
              busy={busy || updates.active}
              onBusy={setBusy}
              onRefresh={refreshOrganization}
              onAdvanced={() => {
                setView('installed');
                setAdvancedInspector(true);
              }}
            />
          )}
          {view === 'library' && libraryMode === 'individual' && (
            <section className="panel">
              <div className="library-controls">
                <label className="search-box">
                  <Search size={17} />
                  <input
                    aria-label="Search mods"
                    placeholder="Search your mods or authors…"
                    value={query}
                    onChange={(e) => setQuery(e.target.value)}
                  />
                </label>
                <select
                  aria-label="Category"
                  value={category}
                  onChange={(e) => setCategory(e.target.value)}
                >
                  {categories.map((c) => (
                    <option key={c}>{c}</option>
                  ))}
                </select>
                <select
                  aria-label="Mod status"
                  value={filter}
                  onChange={(e) => setFilter(e.target.value)}
                >
                  <option value="all">All statuses</option>
                  <option value="imported">Imported</option>
                  <option value="enabled">Enabled</option>
                  <option value="disabled">Disabled</option>
                </select>
              </div>
              <ModTable />
            </section>
          )}
          {view === 'game' && (
            <>
              <div className="two-column">
                <section className="panel padded">
                  <div className="section-heading">
                    <h2>Game installation</h2>
                    <span className="badge imported">{game?.edition ?? 'Not connected'}</span>
                  </div>
                  <h3 className="large-title">Grand Theft Auto V</h3>
                  <p className="subtle">
                    Legacy is supported first. Enhanced and ambiguous folders cannot receive
                    automated installs.
                  </p>
                  <dl>
                    <dt>Directory</dt>
                    <dd className="path">
                      {game?.path ?? 'Select your GTA V folder to get started'}
                    </dd>
                    <dt>Launcher</dt>
                    <dd>{game?.launcher ?? 'Unknown'}</dd>
                    <dt>Build / version</dt>
                    <dd>{game?.version ?? 'Unknown — verify with the mod publisher'}</dd>
                  </dl>
                  <div className="button-row">
                    <button className="button" disabled={busy} onClick={() => void detectGames()}>
                      <RefreshCw size={15} />
                      Detect games
                    </button>
                    <button
                      className="button primary"
                      disabled={busy}
                      onClick={() => void chooseGame()}
                    >
                      <Folder size={15} />
                      Select folder
                    </button>
                  </div>
                </section>
                <section className="panel padded">
                  <ShieldCheck size={27} className="purple-text" />
                  <h2>Single-player comes first.</h2>
                  <p className="subtle spaced">
                    Keep a separate, clean game installation for GTA Online. Zoxryn never changes
                    anti-cheat settings, patches game executables, or launches downloaded
                    installers.
                  </p>
                  <button className="text-button" onClick={() => setSafety(true)}>
                    Read the launch checklist
                    <ArrowRight size={15} />
                  </button>
                  {game?.warnings.map((w) => (
                    <p className="inline-warning" key={w}>
                      <TriangleAlert size={14} />
                      {w}
                    </p>
                  ))}
                </section>
              </div>
              <section className="panel padded dependency-panel">
                <div className="section-heading">
                  <h2>Modding prerequisites</h2>
                  <span className="subtle">Detected locally · never installed silently</span>
                </div>
                {game ? (
                  <div className="dependency-grid">
                    {game.prerequisites.map((p) => (
                      <div className="dependency" key={p.id}>
                        <span className={p.present ? 'dependency-icon present' : 'dependency-icon'}>
                          {p.present ? <Check size={17} /> : <Box size={17} />}
                        </span>
                        <div>
                          <strong>{p.name}</strong>
                          <small>{p.detail}</small>
                        </div>
                        <button
                          className="icon-button"
                          title={`Open official page for ${p.name}`}
                          aria-label={`Open official page for ${p.name}`}
                          onClick={() =>
                            void task(async () => {
                              await call('open_official', { id: p.id });
                            })
                          }
                        >
                          <ExternalLink size={16} />
                        </button>
                      </div>
                    ))}
                  </div>
                ) : (
                  <Empty
                    title="Connect a game to check prerequisites"
                    text="We’ll check common Script Hook V, .NET, OpenIV ASI, RAGE, and LSPDFR markers without changing them."
                    action={() => void chooseGame()}
                    actionLabel="Select GTA folder"
                  />
                )}
              </section>
            </>
          )}
          {view === 'backups' && (
            <>
              <div className="alert">
                <ShieldCheck size={19} />
                <div>
                  <strong>Your way back, one reviewed change at a time.</strong>
                  <p>
                    Restore operations from newest to oldest. External changes block restoration so
                    your edits are preserved. A clean tracked state restores or removes only files
                    managed by Zoxryn; it does not certify the entire game as clean.
                  </p>
                </div>
              </div>
              <section className="panel">
                <div className="section-heading">
                  <h2>Backup history</h2>
                  <span className="count">{data.backups.length}</span>
                </div>
                {data.backups.length ? (
                  <div className="backup-list">
                    {data.backups.map((b) => (
                      <div className="backup-row" key={b.id}>
                        <span className="mod-icon">
                          <Archive size={20} />
                        </span>
                        <div>
                          <strong>{b.mod_name}</strong>
                          <small>
                            {b.operation} · {date(b.created_at)} · {b.files} files ·{' '}
                            {bytes(b.bytes)}
                          </small>
                        </div>
                        <span className="category-label">{b.status.replace('_', ' ')}</span>
                        <button
                          className="button"
                          disabled={busy || b.status !== 'committed'}
                          onClick={() => void restore(b)}
                        >
                          <Undo2 size={14} />
                          Review restore
                        </button>
                      </div>
                    ))}
                  </div>
                ) : (
                  <Empty
                    title="Every change will have a way back"
                    text="Backups appear here after your first tracked operation. They stay in your local data folder between app updates."
                  />
                )}
              </section>
              <section className="panel padded spaced">
                <h2>Restore tracked clean state</h2>
                <p className="subtle">
                  Review a combined file plan for Zoxryn-installed mods in the selected game folder.
                  Originals are restored and new tracked files are removed. Archives, backups, and
                  untracked files are retained. Each mod has its own recoverable transaction.
                  Adopted external entrypoints are excluded; manage those through Installed Mods.
                </p>
                <button
                  className="button"
                  disabled={busy || !installed.some((m) => !m.adopted_from)}
                  onClick={() => void clean()}
                >
                  <ShieldCheck size={15} />
                  Review tracked cleanup
                </button>
              </section>
            </>
          )}
          {view === 'discover' && (
            <>
              <div className="discover-categories" aria-label="Discovery categories">
                {[
                  'Police & LSPDFR',
                  'Superheroes',
                  'Vehicles',
                  'Scripts',
                  'Peds & Skins',
                  'Weapons',
                  'Maps',
                  'Graphics',
                ].map((name, i) => (
                  <button
                    key={name}
                    className="discovery-category"
                    onClick={() =>
                      void task(async () => {
                        await call('open_official', { id: i === 0 ? 'lcpdfr' : 'gta5mods' });
                      })
                    }
                  >
                    <span>{['◈', '✧', '◇', '⌘', '♙', '✣', '⌖', '◌'][i]}</span>
                    <strong>{name}</strong>
                    <small>
                      Browse publisher website <ExternalLink size={11} />
                    </small>
                  </button>
                ))}
              </div>
              <div className="alert">
                <Download size={19} />
                <div>
                  <strong>Discover on the web. Manage right here.</strong>
                  <p>
                    Download directly from the creator’s page, respecting their terms and
                    permissions. Then import the archive. Authorized in-app downloads are planned
                    for Phase 2.
                  </p>
                </div>
              </div>
              <div className="two-column">
                {providers.map((provider, i) => (
                  <section className="panel provider-card" key={provider.id}>
                    <span className="provider-mark">{i === 0 ? '5' : <Shield size={35} />}</span>
                    <span className="pill">TRUSTED COMMUNITY</span>
                    <h2>{provider.name}</h2>
                    <p>{provider.description}</p>
                    <button
                      className="button"
                      onClick={() =>
                        void task(async () => {
                          await call('open_official', { id: provider.id });
                        })
                      }
                    >
                      Visit official website
                      <ArrowUpRight size={16} />
                    </button>
                  </section>
                ))}
              </div>
              <section className="panel padded spaced">
                <h2>Know your archive</h2>
                <div className="format-grid">
                  <div>
                    <span className="badge enabled">ZIP</span>
                    <p>
                      Inspected ASI and explicit scripts-folder layouts can be installed after
                      review.
                    </p>
                  </div>
                  <div>
                    <span className="badge imported">7z / RAR</span>
                    <p>
                      Manual extraction with a trusted tool, then repackage as ZIP. These contents
                      are not inspected here.
                    </p>
                  </div>
                  <div>
                    <span className="badge imported">OIV / DLC / RPF</span>
                    <p>
                      Manual OpenIV mods-folder workflow. No direct edits to original game archives.
                    </p>
                  </div>
                </div>
              </section>
            </>
          )}
          {view === 'settings' && (
            <section className="panel padded settings-panel">
              <section className="neon-settings-section">
                <span className="eyebrow">YOUR WORKSPACE</span>
                <h2>General</h2>
                <p>
                  Local-first management for GTA V Legacy single-player. Every game-file change
                  requires your approval.
                </p>
              </section>
              <AppearanceSettings {...appearance} />
              <PerformanceInfo />
              <UpdateSettings updates={updates} fileBusy={busy || scanning} />
              <div className="settings-row">
                <span className="settings-icon">
                  <Database size={24} />
                </span>
                <div>
                  <h2>Mod library & backups</h2>
                  <p>
                    SQLite records, retained archives, and versioned file backups. Kept outside the
                    application directory so updates preserve your library.
                  </p>
                  <code className="path">{data.data_path}</code>
                  <p>
                    {bytes(data.archive_bytes)} archives · {bytes(data.backup_bytes)} backup &
                    staged data · {bytes(size)} enabled mod files
                  </p>
                </div>
                <button
                  className="button"
                  onClick={() =>
                    void task(async () => {
                      await call('open_location', { kind: 'data' });
                    })
                  }
                >
                  <FolderOpen size={15} />
                  Open data folder
                </button>
              </div>
              <div className="settings-row">
                <span className="settings-icon">
                  <ShieldCheck size={24} />
                </span>
                <div>
                  <h2>Approval before every change</h2>
                  <p>
                    Required. Every install, enable, disable, uninstall, and backup restore opens a
                    file plan for your approval.
                  </p>
                </div>
                <span className="badge enabled">
                  <Check size={12} />
                  Always on
                </span>
              </div>
              <div className="settings-row">
                <span className="settings-icon">
                  <HardDrive size={24} />
                </span>
                <div>
                  <h2>Light on your system</h2>
                  <p>
                    No background polling, automatic downloads, AI models, or account required.
                    Archive work runs off the UI thread.
                  </p>
                </div>
              </div>
              <div className="settings-row">
                <span className="settings-icon">
                  <SlidersHorizontal size={24} />
                </span>
                <div>
                  <h2>GTA V installation</h2>
                  <p>
                    The selected game directory and its launcher are remembered between sessions.
                  </p>
                </div>
                <button className="button" onClick={() => setView('game')}>
                  Manage game
                  <ArrowRight size={15} />
                </button>
              </div>
              <section className="neon-settings-section about-zoxryn">
                <span className="neon-z">Z</span>
                <div>
                  <span className="eyebrow">NEON COLLECTION</span>
                  <h2>About Zoxryn Mods</h2>
                  <p>
                    Version {packageVersion} · Windows 11 x64 · Built for your local mod collection.
                  </p>
                  <p>Original Zoxryn artwork and interface. Tauri, React, Rust & SQLite.</p>
                </div>
              </section>
            </section>
          )}
          <footer className="main-footer">
            <span>
              <Shield size={12} />
              Built for single-player. Designed to keep you in control.
            </span>
            <span>
              ZOXRYN MODS <b>NEON COLLECTION</b>
            </span>
          </footer>
        </main>
      </div>
      {notice && (
        <div
          className={`toast ${notice.error ? 'error' : ''}`}
          role={notice.error ? 'alert' : 'status'}
        >
          {notice.error ? <TriangleAlert size={19} /> : <CheckCheck size={19} />}
          <span>{notice.text}</span>
          <button aria-label="Dismiss notification" onClick={() => setNotice(null)}>
            <X size={16} />
          </button>
        </div>
      )}
      {selected && (
        <div
          className="modal-overlay"
          onClick={(e) => {
            if (e.target === e.currentTarget) setSelected(null);
          }}
        >
          <section
            className="modal mod-inspector"
            role="dialog"
            aria-modal="true"
            aria-label="Mod details"
          >
            <div className="modal-heading">
              <div>
                <div className="eyebrow">ARCHIVE INSPECTION</div>
                <h2>{selected.name}</h2>
              </div>
              {iconButton('Close details', <X size={19} />, () => setSelected(null))}
            </div>
            <div className="modal-body">
              <Thumbnail
                name={selected.name}
                category={selected.category}
                preview={
                  organization.available_members.find(
                    (m) => m.reference.kind === 'managed' && m.reference.id === selected.id,
                  )?.preview ?? null
                }
              />
              <div className="detail-badges">
                <Status status={selected.status} />
                <span className="category-label">{selected.kind}</span>
                <span className="category-label">{selected.edition}</span>
              </div>
              <dl className="detail-grid">
                <div>
                  <dt>Author</dt>
                  <dd>{selected.author}</dd>
                </div>
                <div>
                  <dt>Version</dt>
                  <dd>{selected.version}</dd>
                </div>
                <div>
                  <dt>Imported</dt>
                  <dd>{date(selected.imported_at)}</dd>
                </div>
                <div>
                  <dt>Installed</dt>
                  <dd>{selected.installed_at ? date(selected.installed_at) : 'Not installed'}</dd>
                </div>
                <div>
                  <dt>Category</dt>
                  <dd>{selected.category}</dd>
                </div>
                <div>
                  <dt>Source</dt>
                  <dd>{selected.source}</dd>
                </div>
              </dl>
              {selected.warnings.map((w) => (
                <p className="inline-warning" key={w}>
                  <TriangleAlert size={14} />
                  {w}
                </p>
              ))}
              <h3>Required dependencies</h3>
              <div className="dependency-tags">
                {selected.requirements.length ? (
                  selected.requirements.map((r) => (
                    <span
                      className={`badge ${game?.prerequisites.some((p) => p.id === r && p.present) ? 'enabled' : 'imported'}`}
                      key={r}
                    >
                      {game?.prerequisites.find((p) => p.id === r)?.name ?? r}
                    </span>
                  ))
                ) : (
                  <p className="subtle">
                    None declared or recognized. Review the author’s instructions.
                  </p>
                )}
              </div>
              <h3>
                {selected.files.length ? 'Supported file targets' : 'Manual installation required'}
              </h3>
              <FileGroups files={selected.files.map((f) => f.target)} />
              {selected.instructions && (
                <>
                  <h3>Installation instructions</h3>
                  <pre className="instructions">{selected.instructions}</pre>
                </>
              )}
              <details>
                <summary>Retained archive or adoption backup & SHA-256</summary>
                <code className="path">
                  {selected.archive_path}
                  <br />
                  {selected.archive_hash}
                </code>
              </details>
            </div>
            <div className="modal-footer">
              {selected.status !== 'imported' && (
                <button
                  className="button danger-button"
                  disabled={busy || !game}
                  onClick={() => void operation(selected, 'remove')}
                >
                  <Trash2 size={15} />
                  Review uninstall
                </button>
              )}
              <span className="spacer" />
              <button className="button" onClick={() => setSelected(null)}>
                Close
              </button>
              {selected.status === 'imported' ? (
                <button
                  className="button primary"
                  disabled={busy || !game || !selected.files.length}
                  onClick={() => void operation(selected, 'install')}
                >
                  Review install
                  <ArrowRight size={15} />
                </button>
              ) : (
                <button
                  className="button primary"
                  disabled={busy || !game}
                  onClick={() =>
                    void operation(selected, selected.status === 'enabled' ? 'disable' : 'enable')
                  }
                >
                  Review {selected.status === 'enabled' ? 'disable' : 'enable'}
                  <ArrowRight size={15} />
                </button>
              )}
            </div>
          </section>
        </div>
      )}
      {plan && (
        <div className="modal-overlay">
          <section
            className="modal wide"
            role="dialog"
            aria-modal="true"
            aria-label="Review file changes"
          >
            <div className="modal-heading">
              <div>
                <div className="eyebrow">YOU’RE IN CONTROL</div>
                <h2>
                  Review{' '}
                  {plan.operation === 'remove'
                    ? planIsRelease
                      ? 'release of control'
                      : 'uninstall'
                    : plan.operation === 'clean'
                      ? 'tracked cleanup'
                      : plan.operation}{' '}
                  plan
                </h2>
              </div>
              {iconButton('Cancel plan', <X size={19} />, () => setPlan(null))}
            </div>
            <div className="modal-body">
              <p className="subtle">
                Only these tracked files will change. Existing files are backed up before writing,
                then every result is verified.
              </p>
              <code className="path plan-path">{plan.game.path}</code>
              {plan.blockers.length > 0 && (
                <div className="alert danger">
                  <TriangleAlert size={18} />
                  <div>
                    <strong>This operation is blocked</strong>
                    {plan.blockers.map((b, i) => (
                      <p key={i}>{b}</p>
                    ))}
                  </div>
                </div>
              )}
              <div className="file-list planned">
                {plan.files.map((f) => (
                  <div key={f.target}>
                    <FileCheck2 size={15} />
                    <code>{f.target}</code>
                    <span className="category-label">{f.action}</span>
                  </div>
                ))}
              </div>
              {plan.warnings.map((w) => (
                <p className="inline-warning" key={w}>
                  <TriangleAlert size={14} />
                  {w}
                </p>
              ))}
              <div className="approval-box">
                {(plan.operation === 'adopt' || plan.independent_scope) && (
                  <label>
                    <input
                      type="checkbox"
                      checked={ownershipReviewed}
                      onChange={(e) => setOwnershipReviewed(e.target.checked)}
                    />
                    {plan.independent_scope
                      ? 'I checked the author’s instructions and confirmed this complete suit package is independent, is not required by other mods, and needs no unlisted resources or registration edits. If uncertain, I will cancel.'
                      : 'I confirmed the complete listed entrypoint is independent, is not shared by other mods, and needs no unlisted companion-file management. If uncertain, I will cancel.'}
                  </label>
                )}
                <label>
                  <input
                    type="checkbox"
                    checked={reviewed}
                    onChange={(e) => setReviewed(e.target.checked)}
                  />
                  I closed GTA V and checked the author’s instructions and compatibility with this
                  Legacy game build.
                </label>
                <label>
                  <input
                    type="checkbox"
                    checked={approved}
                    onChange={(e) => setApproved(e.target.checked)}
                  />
                  I approve this exact file plan. I understand that remove, disable, and restore may
                  remove tracked mod files.
                </label>
              </div>
            </div>
            <div className="modal-footer">
              <span className="subtle">{plan.files.length} tracked files · backups retained</span>
              <span className="spacer" />
              <button className="button" disabled={busy} onClick={() => setPlan(null)}>
                Cancel
              </button>
              <button
                className="button primary"
                disabled={
                  busy ||
                  !approved ||
                  !reviewed ||
                  plan.blockers.length > 0 ||
                  ((plan.operation === 'adopt' || !!plan.independent_scope) && !ownershipReviewed)
                }
                onClick={() => void apply()}
              >
                {busy ? <LoaderCircle className="spin" size={15} /> : <ShieldCheck size={15} />}
                Approve &{' '}
                {plan.operation === 'remove'
                  ? planIsRelease
                    ? 'release of control'
                    : 'uninstall'
                  : plan.operation === 'clean'
                    ? 'tracked cleanup'
                    : plan.operation}
              </button>
            </div>
          </section>
        </div>
      )}
      {games && (
        <div className="modal-overlay">
          <section
            className="modal"
            role="dialog"
            aria-modal="true"
            aria-label="Detected GTA installations"
          >
            <div className="modal-heading">
              <h2>{games.length ? 'Choose your installation' : 'No installation detected'}</h2>
              {iconButton('Close game detection', <X size={19} />, () => setGames(null))}
            </div>
            <div className="modal-body">
              <p className="subtle">
                Detection is read-only. Select Legacy for automated mod management.
              </p>
              {games.map((g) => (
                <button
                  className="game-choice"
                  disabled={busy}
                  key={g.path}
                  onClick={() =>
                    void task(async () => {
                      await call('select_game', { path: g.path, launcher: g.launcher });
                      setGames(null);
                      await refresh();
                    })
                  }
                >
                  <Gamepad2 size={23} />
                  <span>
                    <strong>
                      GTA V {g.edition} · {g.launcher}
                    </strong>
                    <small className="path">{g.path}</small>
                  </span>
                  <ChevronRight size={18} />
                </button>
              ))}
            </div>
            <div className="modal-footer">
              <button className="button" onClick={() => setGames(null)}>
                Close
              </button>
              <button className="button primary" disabled={busy} onClick={() => void chooseGame()}>
                <Folder size={15} />
                Choose folder manually
              </button>
            </div>
          </section>
        </div>
      )}
      {safety && (
        <div className="modal-overlay">
          <section
            className="modal"
            role="dialog"
            aria-modal="true"
            aria-label="Single-player safety checklist"
          >
            <div className="modal-heading">
              <h2>Before you play</h2>
              {iconButton('Close safety checklist', <X size={19} />, () => setSafety(false))}
            </div>
            <div className="modal-body safety-content">
              <ShieldCheck size={36} className="purple-text" />
              <h3>Make room for a better single-player.</h3>
              <ol>
                <li>Choose the Legacy game folder and check your game build.</li>
                <li>
                  Get prerequisites from their publishers. Confirm compatible versions yourself;
                  file presence alone is insufficient.
                </li>
                <li>Download a mod you trust. Read its instructions and import the archive.</li>
                <li>
                  Review dependencies, conflicts, and the proposed file changes. Approve only what
                  you understand.
                </li>
                <li>
                  Before launching any modded game, keep it in single-player. Never use this modded
                  installation for GTA Online.
                </li>
              </ol>
              <div className="alert">
                <TriangleAlert size={18} />
                <p>
                  Zoxryn does not launch GTA V in Phase 1. Launch manually after this checklist. It
                  never bypasses anti-cheat or changes security settings. Tracked cleanup is not
                  proof that the whole installation is safe for GTA Online.
                </p>
              </div>
            </div>
            <div className="modal-footer">
              <button className="button primary" onClick={() => setSafety(false)}>
                Got it
                <Check size={15} />
              </button>
            </div>
          </section>
        </div>
      )}
      {report && (
        <div className="modal-overlay">
          <section className="modal" role="dialog" aria-modal="true" aria-label="Operation report">
            <div className="modal-heading">
              <h2>Changes verified.</h2>
              {iconButton('Close operation report', <X size={19} />, () => setReport(null))}
            </div>
            <div className="modal-body">
              <div className="report-icon">
                <PackageCheck size={35} />
              </div>
              <p>{report.message}</p>
              <div className="file-list">
                {report.files.map((f) => (
                  <div key={f}>
                    <Check size={15} />
                    <code>{f}</code>
                  </div>
                ))}
              </div>
              <p className="subtle">
                Transaction <code>{report.id}</code>
              </p>
              <p className="subtle">
                Review Backups & recovery for your restore history. The original downloaded archive
                is retained.
              </p>
            </div>
            <div className="modal-footer">
              <button className="button primary" onClick={() => setReport(null)}>
                Done
                <Check size={15} />
              </button>
            </div>
          </section>
        </div>
      )}
    </div>
  );
}
