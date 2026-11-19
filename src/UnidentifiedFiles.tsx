import { useDeferredValue, useMemo, useState } from 'react';
import { FileSearch, Search, ShieldCheck, X, FolderPlus, UserCheck } from 'lucide-react';
import type { Component, OrganizationSnapshot, Suit } from './organizationTypes';
import { suitCategories } from './organizationTypes';
import { call } from './api';
import FileGroups from './FileGroups';
const evidencePath = (item: Component | Suit) =>
  item.files[0] ?? ('location' in item ? item.location : '');

interface Props {
  data: OrganizationSnapshot;
  busy: boolean;
  onBusy: (v: boolean) => void;
  onRefresh: () => Promise<void>;
  onAdvanced: () => void;
}
export default function UnidentifiedFiles({ data, busy, onBusy, onRefresh, onAdvanced }: Props) {
  const [query, setQuery] = useState('');
  const deferred = useDeferredValue(query);
  const [page, setPage] = useState(0);
  const [selected, setSelected] = useState<Component | Suit | null>(null);
  const [target, setTarget] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [confirm, setConfirm] = useState(false);
  const [name, setName] = useState('');
  const [category, setCategory] = useState('Custom Peds');
  const [ack, setAck] = useState(false);
  const entries = useMemo(() => [...(data.unidentified ?? []), ...(data.candidates ?? [])], [data]);
  const filtered = useMemo(
    () =>
      entries.filter((e) =>
        `${e.name} ${e.files.join(' ')}`.toLowerCase().includes(deferred.toLowerCase()),
      ),
    [entries, deferred],
  );
  const max = Math.max(0, Math.ceil(filtered.length / 40) - 1),
    current = Math.min(page, max);
  async function mutate(fn: () => Promise<unknown>) {
    onBusy(true);
    setError(null);
    try {
      await fn();
      await onRefresh();
      setConfirm(false);
      setSelected(null);
    } catch (e) {
      setError(String(e));
    } finally {
      onBusy(false);
    }
  }
  async function assign() {
    if (!selected || !('reference' in selected)) return;
    const group = data.collections.find((c) => c.id === target);
    if (!group) return;
    const members = [...group.members.map((m) => m.reference), selected.reference].filter(
      (r, i, a) => a.findIndex((x) => x.kind === r.kind && x.id === r.id) === i,
    );
    await mutate(() =>
      group.automatic
        ? call('save_virtual_group', {
            id: group.id,
            name: group.name,
            category: group.category,
            members,
            confirmed: false,
          })
        : call('save_collection', {
            id: group.id,
            name: group.name,
            category: group.category,
            description: group.description,
            members,
          }),
    );
  }
  return (
    <section className="unidentified-page">
      <div className="scan-overview panel">
        <span className="scan-symbol">
          <FileSearch size={28} />
        </span>
        <div>
          <span className="eyebrow">EVIDENCE, KEPT INTACT</span>
          <h2>Unidentified Files</h2>
          <p>
            Unresolved files and resource clues stay here. They are not independent mods or complete
            outfits. Virtual assignments never move files or grant ownership.
          </p>
        </div>
        <button className="button" onClick={onAdvanced}>
          Advanced File Inspector
        </button>
      </div>
      <div className="org-filters">
        <label className="search-box">
          <Search size={17} />
          <input
            aria-label="Search unidentified files"
            placeholder="Search original filenames and paths…"
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setPage(0);
            }}
          />
        </label>
        <span>{filtered.length} unresolved evidence entries</span>
      </div>
      {error && (
        <p className="inline-warning" role="alert">
          {error}
        </p>
      )}
      <div className={`unidentified-workspace ${selected ? 'with-inspector' : ''}`}>
        <div className="panel evidence-table">
          <table>
            <thead>
              <tr>
                <th>Original file / evidence</th>
                <th>Location & parent hints</th>
                <th>Confidence</th>
              </tr>
            </thead>
            <tbody>
              {filtered.slice(current * 40, current * 40 + 40).map((item) => {
                const id = 'reference' in item ? item.reference.id : item.id;
                return (
                  <tr key={id}>
                    <td>
                      <button
                        className="mod-link"
                        onClick={() => {
                          setSelected(item);
                          setTarget('');
                          setConfirm(false);
                        }}
                      >
                        {item.files[0]?.split('/').at(-1) ?? item.name}
                        <small>{'role' in item ? item.role : 'Unverified resource clue'}</small>
                      </button>
                    </td>
                    <td>
                      <code>{evidencePath(item) || 'No confirmed location'}</code>
                      <small>
                        Parent hint:{' '}
                        {evidencePath(item).split('/').slice(0, -1).join('/') || 'GTA V root'}
                      </small>
                    </td>
                    <td>
                      <span className="badge unknown">Unidentified</span>
                      <small>Identity and compatibility unverified</small>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
          {!filtered.length && (
            <div className="org-empty">
              <ShieldCheck size={30} />
              <h3>No unresolved evidence in this view</h3>
              <p>All raw scan records remain accessible in the Advanced File Inspector.</p>
            </div>
          )}
          <div className="org-pagination">
            <button className="button" disabled={!current} onClick={() => setPage(current - 1)}>
              Previous
            </button>
            <span>
              Page {current + 1} of {max + 1}
            </span>
            <button
              className="button"
              disabled={current === max}
              onClick={() => setPage(current + 1)}
            >
              Next
            </button>
          </div>
        </div>
        {selected && (
          <aside className="org-detail" aria-label="Unidentified file details">
            <div className="org-detail-title">
              <span className="eyebrow">ADVANCED EVIDENCE</span>
              <button
                className="icon-button"
                aria-label="Close unidentified details"
                onClick={() => setSelected(null)}
              >
                <X size={18} />
              </button>
            </div>
            <h3>{selected.name}</h3>
            <p>{selected.control_reason}</p>
            <p className="subtle">
              No reliable complete-package identity or supported registration/ownership relationship
              has been established. A filename or file extension alone is not sufficient.
            </p>
            <FileGroups files={selected.files} />
            {'reference' in selected ? (
              <>
                <h4>Assign a virtual parent</h4>
                <p>Organizational reference only. Original scan evidence is retained.</p>
                <select
                  aria-label="Assign to collection"
                  value={target}
                  onChange={(e) => setTarget(e.target.value)}
                >
                  <option value="">Choose a collection</option>
                  {data.collections.map((c) => (
                    <option key={c.id} value={c.id}>
                      {c.name}
                    </option>
                  ))}
                </select>
                <button className="button" disabled={busy || !target} onClick={() => void assign()}>
                  <FolderPlus size={15} />
                  Assign reference
                </button>
              </>
            ) : (
              <>
                <button
                  className="button"
                  disabled={busy}
                  onClick={() => {
                    setConfirm(true);
                    setName('');
                    setAck(false);
                  }}
                >
                  <UserCheck size={15} />
                  Confirm an outfit identity
                </button>
                <p className="subtle">
                  Human identification only; unsafe management controls remain blocked.
                </p>
              </>
            )}
            {confirm && 'id' in selected && (
              <form
                className="identity-editor"
                onSubmit={(e) => {
                  e.preventDefault();
                  void mutate(() =>
                    call('confirm_skin_identity', {
                      id: selected.id,
                      name,
                      category,
                      confirmed: true,
                    }),
                  );
                }}
              >
                <label>
                  Outfit name
                  <input
                    aria-label="Confirmed outfit name"
                    value={name}
                    maxLength={120}
                    required
                    onChange={(e) => setName(e.target.value)}
                  />
                </label>
                <label>
                  Category
                  <select
                    aria-label="Confirmed outfit category"
                    value={category}
                    onChange={(e) => setCategory(e.target.value)}
                  >
                    {suitCategories.map((c) => (
                      <option key={c}>{c}</option>
                    ))}
                  </select>
                </label>
                <label className="approval-inline">
                  <input type="checkbox" checked={ack} onChange={(e) => setAck(e.target.checked)} />
                  I have identified this outfit from its creator's package or documentation. This
                  confirms its label, not compatibility or safe enable/disable.
                </label>
                <button className="button primary" disabled={busy || !ack || !name.trim()}>
                  Save identified outfit
                </button>
              </form>
            )}
          </aside>
        )}
      </div>
    </section>
  );
}
