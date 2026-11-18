import { useEffect, useMemo, useState } from 'react';
import {
  Archive,
  ArrowRight,
  Box,
  Check,
  ChevronLeft,
  ChevronRight,
  Clock3,
  FileCheck2,
  FolderSearch,
  Info,
  LoaderCircle,
  MoreHorizontal,
  Power,
  Search,
  ShieldCheck,
  TriangleAlert,
  X,
} from 'lucide-react';
import type { DiscoveredMod, Game, Mod, ScanInventory } from './types';
const size = (n: number) =>
  n >= 1048576
    ? `${(n / 1048576).toFixed(1)} MB`
    : n >= 1024
      ? `${(n / 1024).toFixed(1)} KB`
      : `${n} B`;
const statusName = (s: string) =>
  s === 'needs_review' ? 'Needs Review' : s[0].toUpperCase() + s.slice(1);
const paths = (p: string) => p.replace(/^\\\\\?\\UNC\\/, '\\\\').replace(/^\\\\\?\\/, '');
type Row = DiscoveredMod & { managed?: Mod };
export function inventoryRows(inventory: ScanInventory | null | undefined, mods: Mod[]): Row[] {
  if (!inventory) return [];
  const external = inventory.items.filter((i) => !i.managed_id);
  const records = new Map(mods.map((m) => [m.id, m]));
  const managed = inventory.managed.map((item) => {
    const record = records.get(item.id);
    return {
      id: item.id,
      game_path: inventory.game_path,
      name: item.name,
      category: item.category,
      status: item.status,
      source: item.source || 'Zoxryn',
      confidence: 'identified',
      kind: item.adopted ? 'Adopted entrypoint' : 'Zoxryn-managed mod',
      location: item.files[0] ?? '',
      files: item.files.map((path) => ({
        path,
        size: record?.files.find((f) => f.target === path)?.size ?? 0,
        modified_ns: '',
      })),
      dependencies: item.dependencies,
      conflicts: item.status === 'needs_review' ? item.notes : [],
      notes: item.notes,
      adoptable: false,
      control_reason: item.adopted
        ? 'Independent entrypoint under verified, reversible control. Release control restores the adoption-time file.'
        : 'Managed through the existing verified backup and ownership ledger.',
      managed_id: item.id,
      rpf: null,
      managed: record,
    } as Row;
  });
  return [...managed, ...external];
}
export function inventoryCounts(inventory: ScanInventory | null | undefined, mods: Mod[]) {
  const rows = inventoryRows(inventory, mods);
  const components = rows.filter(
    (r) => r.confidence === 'identified' && r.kind !== 'Shared prerequisite',
  );
  return {
    total: components.length,
    enabled: components.filter((r) => r.status === 'enabled').length,
    disabled: components.filter((r) => r.status === 'disabled').length,
    openiv: rows.filter((r) => r.source === 'OpenIV' && r.kind !== 'OpenIV folder').length,
    review: rows.filter((r) => r.status === 'needs_review' || r.status === 'unknown').length,
    items: rows.length,
  };
}
interface Props {
  inventory: ScanInventory | null | undefined;
  mods: Mod[];
  game: Game | null;
  scanning: boolean;
  busy: boolean;
  onRescan: () => void;
  onAdopt: (id: string) => void;
  onOperation: (mod: Mod, operation: string) => void;
}
export default function InstalledMods({
  inventory,
  mods,
  game,
  scanning,
  busy,
  onRescan,
  onAdopt,
  onOperation,
}: Props) {
  const [query, setQuery] = useState('');
  const [category, setCategory] = useState('all');
  const [status, setStatus] = useState('all');
  const [source, setSource] = useState('all');
  const [sort, setSort] = useState('name');
  const [scope, setScope] = useState('all');
  const [page, setPage] = useState(0);
  const [detail, setDetail] = useState<Row | null>(null);
  const rows = useMemo(() => inventoryRows(inventory, mods), [inventory, mods]);
  const categories = useMemo(
    () =>
      [
        ...new Set(rows.flatMap((r) => [r.category, ...Object.keys(r.rpf?.categories ?? {})])),
      ].sort(),
    [rows],
  );
  const filtered = useMemo(
    () =>
      rows
        .filter(
          (r) =>
            (category === 'all' ||
              r.category === category ||
              Object.hasOwn(r.rpf?.categories ?? {}, category)) &&
            (status === 'all' || r.status === status) &&
            (source === 'all' || r.source === source) &&
            (scope === 'all' || (scope === 'managed' ? !!r.managed : r.confidence === scope)) &&
            `${r.name} ${r.location} ${r.kind} ${r.rpf?.samples.join(' ') ?? ''}`
              .toLowerCase()
              .includes(query.toLowerCase()),
        )
        .sort((a, b) =>
          sort === 'status'
            ? a.status.localeCompare(b.status) || a.name.localeCompare(b.name)
            : sort === 'location'
              ? a.location.localeCompare(b.location)
              : sort === 'size'
                ? b.files.reduce((n, f) => n + f.size, 0) - a.files.reduce((n, f) => n + f.size, 0)
                : a.name.localeCompare(b.name),
        ),
    [rows, query, category, status, source, sort, scope],
  );
  useEffect(() => setPage(0), [query, category, status, source, sort, scope, inventory]);
  useEffect(() => {
    if (!detail) return;
    const dialog = document.querySelector<HTMLElement>('[aria-label="Installed mod details"]');
    const previous = document.activeElement as HTMLElement | null;
    const controls = () => [
      ...(dialog?.querySelectorAll<HTMLElement>('button:not(:disabled),summary') ?? []),
    ];
    controls()[0]?.focus();
    const key = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setDetail(null);
      if (e.key === 'Tab') {
        const all = controls();
        if (e.shiftKey && document.activeElement === all[0]) {
          e.preventDefault();
          all.at(-1)?.focus();
        } else if (!e.shiftKey && document.activeElement === all.at(-1)) {
          e.preventDefault();
          all[0]?.focus();
        }
      }
    };
    window.addEventListener('keydown', key);
    return () => {
      window.removeEventListener('keydown', key);
      previous?.isConnected && previous.focus();
    };
  }, [detail]);
  const pages = Math.max(1, Math.ceil(filtered.length / 50));
  const shown = filtered.slice(page * 50, (page + 1) * 50);
  const canScan = game?.edition === 'Legacy';
  const action = (row: Row) =>
    row.managed ? (
      <button
        className="button compact"
        disabled={busy || scanning}
        onClick={() => {
          setDetail(null);
          onOperation(row.managed!, row.managed!.status === 'enabled' ? 'disable' : 'enable');
        }}
      >
        <Power size={13} />
        Review {row.managed.status === 'enabled' ? 'disable' : 'enable'}
      </button>
    ) : (
      <button
        className="button compact"
        disabled={busy || scanning || !row.adoptable || !!inventory?.stale}
        title={row.control_reason}
        onClick={() => {
          setDetail(null);
          onAdopt(row.id);
        }}
      >
        <ShieldCheck size={13} />
        Review adoption
      </button>
    );
  return (
    <div className="installed-workspace">
      <section className="scan-overview panel">
        <span className="scan-symbol">
          <FolderSearch size={28} />
        </span>
        <div>
          <div className="eyebrow">READ-ONLY INVENTORY</div>
          <h2>Everything discoverable. Clearly accounted for.</h2>
          <p>
            Recognized components, OpenIV containers, and unknown files stay distinct. Scanning
            never changes your game.
          </p>
        </div>
        <button
          className="button primary"
          disabled={!canScan || scanning || busy}
          onClick={onRescan}
        >
          {scanning ? <LoaderCircle className="spin" size={15} /> : <FolderSearch size={15} />}
          {scanning ? 'Scanning…' : inventory ? 'Rescan' : 'Scan Installed Mods'}
        </button>
      </section>
      {!canScan && (
        <div className="alert">
          <Info size={18} />
          <p>
            Select a valid GTA V Legacy installation in Game & dependencies to scan installed mods.
          </p>
        </div>
      )}
      {inventory && (
        <div className="scan-meta" role="status">
          <span>
            <Clock3 size={13} />
            {new Date(inventory.scanned_at * 1000).toLocaleString()}
          </span>
          <span>
            {inventory.visited_files.toLocaleString()} files · {inventory.elapsed_ms} ms
          </span>
          <span>{size(inventory.bytes_read)} table data read · no full RPF hashes</span>
          <span className={inventory.complete && !inventory.stale ? 'scan-good' : 'scan-warning'}>
            {inventory.stale
              ? 'Results need a rescan'
              : inventory.complete
                ? 'Directory scan complete'
                : 'Partial scan — review warnings'}
          </span>
        </div>
      )}
      {scanning && (
        <div className="alert scan-progress" role="status">
          <LoaderCircle className="spin" size={17} />
          <p>
            Scanning in a background worker. You can keep browsing; no files are being installed or
            disabled.
          </p>
        </div>
      )}
      {inventory?.stale && (
        <div className="alert">
          <TriangleAlert size={18} />
          <p>
            Game state changed after this inventory was taken. External results are historical until
            you rescan; adoption is blocked.
          </p>
        </div>
      )}
      {!!inventory?.warnings.length && (
        <details className="scan-warnings">
          <summary>
            {inventory.warnings.length} scan warning(s) · {inventory.skipped} skipped entries
          </summary>
          {inventory.warnings.map((w, i) => (
            <p key={i}>{w}</p>
          ))}
        </details>
      )}
      <section className="panel installed-list">
        <div className="inventory-tabs" aria-label="Inventory scope">
          {[
            ['all', 'All detected'],
            ['identified', 'Identified components'],
            ['possible', 'Possible modifications'],
            ['unknown', 'Unknown files'],
            ['managed', 'Managed by Zoxryn'],
          ].map(([value, label]) => (
            <button
              key={value}
              className={scope === value ? 'active' : ''}
              onClick={() => setScope(value)}
            >
              {label}
            </button>
          ))}
        </div>
        <div className="installed-controls">
          <label className="search-box">
            <Search size={16} />
            <input
              aria-label="Search installed mods"
              placeholder="Search name, location, or RPF resource…"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
          </label>
          <select
            aria-label="Installed category"
            value={category}
            onChange={(e) => setCategory(e.target.value)}
          >
            <option value="all">All categories</option>
            {categories.map((c) => (
              <option key={c}>{c}</option>
            ))}
          </select>
          <select
            aria-label="Installed status"
            value={status}
            onChange={(e) => setStatus(e.target.value)}
          >
            <option value="all">All statuses</option>
            {['enabled', 'disabled', 'unknown', 'needs_review'].map((s) => (
              <option key={s} value={s}>
                {statusName(s)}
              </option>
            ))}
          </select>
          <select
            aria-label="Installation source"
            value={source}
            onChange={(e) => setSource(e.target.value)}
          >
            <option value="all">All sources</option>
            {['Zoxryn', 'OpenIV', 'Manual', 'Unknown'].map((s) => (
              <option key={s}>{s}</option>
            ))}
          </select>
          <select
            aria-label="Sort installed mods"
            value={sort}
            onChange={(e) => setSort(e.target.value)}
          >
            <option value="name">Name A–Z</option>
            <option value="status">Status</option>
            <option value="location">Location</option>
            <option value="size">Size</option>
          </select>
        </div>
        {shown.length ? (
          <div className="table-wrap">
            <table className="installed-table">
              <thead>
                <tr>
                  <th>MOD / COMPONENT</th>
                  <th>STATUS & SOURCE</th>
                  <th>LOCATION</th>
                  <th>CONTROL</th>
                </tr>
              </thead>
              <tbody>
                {shown.map((row) => (
                  <tr key={row.id}>
                    <td>
                      <button className="installed-name" onClick={() => setDetail(row)}>
                        <span className={`mod-icon ${row.source === 'OpenIV' ? 'openiv' : ''}`}>
                          {row.rpf ? <Archive size={18} /> : <Box size={18} />}
                        </span>
                        <span>
                          <strong>{row.name}</strong>
                          <small>
                            {row.category} · {row.kind}
                          </small>
                          <em className={`confidence ${row.confidence}`}>
                            {row.managed
                              ? 'Verified ownership ledger'
                              : row.confidence === 'identified'
                                ? 'Recognized component'
                                : row.confidence === 'possible'
                                  ? 'Possible modification'
                                  : 'Unidentified file'}
                          </em>
                        </span>
                      </button>
                    </td>
                    <td>
                      <span className={`badge ${row.status}`}>
                        <i />
                        {statusName(row.status)}
                      </span>
                      <small className="source-label">
                        {row.source}
                        {row.managed ? ' · managed' : ''}
                      </small>
                    </td>
                    <td>
                      <code className="file-location">{row.location}</code>
                      <small className="source-label">
                        {row.files.length
                          ? size(row.files.reduce((n, f) => n + f.size, 0))
                          : 'Container · size not aggregated'}
                        {row.rpf ? ` · ${row.rpf.state}` : ''}
                      </small>
                    </td>
                    <td>
                      <div className="installed-actions">
                        {action(row)}
                        <button
                          className="icon-button"
                          aria-label={`Details for installed ${row.name}`}
                          onClick={() => setDetail(row)}
                        >
                          <MoreHorizontal size={17} />
                        </button>
                      </div>
                      {!row.managed && !row.adoptable && (
                        <small className="control-note">
                          {row.kind.includes('RPF')
                            ? 'Container cannot be safely isolated'
                            : 'Manual review required'}
                        </small>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <div className="empty">
            <div className="empty-icon">
              <FolderSearch size={28} />
            </div>
            <h3>
              {inventory ? 'No matching installed items' : 'See what’s already in Los Santos'}
            </h3>
            <p>
              {inventory
                ? 'Try another search, category, source, or status.'
                : 'Select your Legacy game folder, then scan its existing mods. Importing archives is not required.'}
            </p>
          </div>
        )}
        <div className="inventory-pagination">
          <span>{filtered.length.toLocaleString()} items · showing up to 50 per page</span>
          <button
            className="icon-button"
            disabled={page === 0}
            aria-label="Previous installed page"
            onClick={() => setPage((p) => p - 1)}
          >
            <ChevronLeft size={16} />
          </button>
          <span>
            {page + 1} / {pages}
          </span>
          <button
            className="icon-button"
            disabled={page + 1 >= pages}
            aria-label="Next installed page"
            onClick={() => setPage((p) => p + 1)}
          >
            <ChevronRight size={16} />
          </button>
        </div>
      </section>
      <div className="scan-footnote">
        <ShieldCheck size={16} />
        <p>
          Enabled means a recognized loading location, not proof that a mod ran. Container/resource
          counts are not counts of individual creator mods. Unknown original RPF files are listed
          without reading their contents or asserting they were modified.
        </p>
      </div>
      {detail && (
        <div className="modal-overlay">
          <section
            className="modal wide"
            role="dialog"
            aria-modal="true"
            aria-label="Installed mod details"
          >
            <div className="modal-heading">
              <div>
                <div className="eyebrow">
                  {detail.managed ? 'MANAGED FILES' : 'DISCOVERED ON DISK'}
                </div>
                <h2>{detail.name}</h2>
              </div>
              <button
                className="icon-button"
                aria-label="Close installed details"
                onClick={() => setDetail(null)}
              >
                <X size={19} />
              </button>
            </div>
            <div className="modal-body">
              <div className="detail-badges">
                <span className={`badge ${detail.status}`}>{statusName(detail.status)}</span>
                <span className="category-label">{detail.source}</span>
                <span className="category-label">{detail.confidence}</span>
                <span className="category-label">{detail.category}</span>
              </div>
              <h3>File location</h3>
              <code className="path">
                {paths(detail.game_path)}/{detail.location}
              </code>
              <h3>Control & ownership</h3>
              <p className="subtle">{detail.control_reason}</p>
              {detail.notes.map((n, i) => (
                <p className="subtle" key={i}>
                  {n}
                </p>
              ))}
              {detail.conflicts.map((c, i) => (
                <p className="inline-warning" key={i}>
                  <TriangleAlert size={14} />
                  {c}
                </p>
              ))}
              <h3>Dependencies & potential shared files</h3>
              <div className="dependency-tags">
                {detail.dependencies.length ? (
                  detail.dependencies.map((id) => (
                    <span
                      key={id}
                      className={`badge ${game?.prerequisites.some((p) => p.id === id && p.present) ? 'enabled' : 'needs_review'}`}
                    >
                      {game?.prerequisites.find((p) => p.id === id)?.name ?? id}
                    </span>
                  ))
                ) : (
                  <p className="subtle">
                    No dependencies established. File presence cannot prove independent ownership.
                  </p>
                )}
              </div>
              <h3>Discovered files</h3>
              {!detail.files.length && (
                <p className="subtle">
                  Directory/container record. Inspect the child file and RPF rows; individual
                  ownership and an aggregate size are not established.
                </p>
              )}
              <div className="file-list">
                {detail.files.map((f) => (
                  <div key={f.path}>
                    <FileCheck2 size={14} />
                    <code>{f.path}</code>
                    <small>{size(f.size)}</small>
                  </div>
                ))}
              </div>
              {detail.rpf && (
                <section className="rpf-evidence">
                  <h3>RPF inspection · {detail.rpf.state}</h3>
                  <p className="subtle">
                    {detail.rpf.entries.toLocaleString()} TOC entries ·{' '}
                    {size(detail.rpf.bytes_read)} read. Resource category clues:
                  </p>
                  <div className="dependency-tags">
                    {Object.entries(detail.rpf.categories).map(([c, n]) => (
                      <span className="category-label" key={c}>
                        {c}: {n}
                      </span>
                    ))}
                  </div>
                  <div className="rpf-samples">
                    {detail.rpf.samples.map((p) => (
                      <code key={p}>{p}</code>
                    ))}
                  </div>
                  {detail.rpf.notes.map((n, i) => (
                    <p className="inline-warning" key={i}>
                      <Info size={14} />
                      {n}
                    </p>
                  ))}
                </section>
              )}
            </div>
            <div className="modal-footer">
              {detail.managed?.adopted_from && (
                <button
                  className="button"
                  disabled={busy || scanning}
                  onClick={() => {
                    setDetail(null);
                    onOperation(detail.managed!, 'remove');
                  }}
                >
                  Review release of control
                </button>
              )}
              <span className="spacer" />
              <button className="button" onClick={() => setDetail(null)}>
                Close
              </button>
              {action(detail)}
            </div>
          </section>
        </div>
      )}
    </div>
  );
}
