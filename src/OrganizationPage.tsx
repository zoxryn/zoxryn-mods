import { useDeferredValue, useEffect, useId, useMemo, useRef, useState } from 'react';
import {
  Boxes,
  ChevronRight,
  Grid2X2,
  ImagePlus,
  List,
  LoaderCircle,
  Pencil,
  Plus,
  Search,
  ShieldCheck,
  Trash2,
  TriangleAlert,
  X,
  Car,
  Shield,
  Shirt,
  Users,
  Sparkles,
} from 'lucide-react';
import { call, pick } from './api';
import Thumbnail from './Thumbnail';
import FileGroups from './FileGroups';
import type {
  Collection,
  Component,
  MemberRef,
  OrganizationSnapshot,
  Suit,
} from './organizationTypes';
import { suitCategories } from './organizationTypes';
import './OrganizationPage.css';

interface Props {
  focusId?: string | null;
  mode: 'collections' | 'suits';
  data: OrganizationSnapshot;
  loading: boolean;
  error: string | null;
  busy: boolean;
  onRefresh: () => Promise<void>;
  onBusy: (busy: boolean) => void;
  onReviewSuit: (id: string, operation: string) => void;
  onParent: (id: string) => void;
}
const refKey = (m: MemberRef) => `${m.kind}:${m.id}`;
const label = (value: string) => value.replaceAll('_', ' ').replace(/^./, (c) => c.toUpperCase());
function Badge({ status }: { status: string }) {
  const text = status === 'unknown' ? 'Detected' : label(status);
  const meaning =
    status === 'unknown'
      ? 'Detected only; enabled state and compatibility are unverified.'
      : status === 'enabled'
        ? 'Enabled loading files or managed package; compatibility remains unverified.'
        : status === 'disabled'
          ? 'Disabled package or loading entry; this is not an in-game compatibility check.'
          : status === 'needs_review'
            ? 'Needs review: inspect membership, dependencies and management restrictions before any file operation.'
            : 'See component details for the recorded state.';
  return (
    <span className={`badge ${status}`} title={meaning} aria-label={`${text}: ${meaning}`}>
      {text}
    </span>
  );
}
function OutfitFiles({ files }: { files: string[] }) {
  const [open, setOpen] = useState(false);
  return (
    <details className="org-expand" onToggle={(e) => setOpen(e.currentTarget.open)}>
      <summary>Outfit components ({files.length})</summary>
      {open && <FileGroups files={files} />}
    </details>
  );
}
function Files({ files }: { files: string[] }) {
  const [limit, setLimit] = useState(40);
  return (
    <>
      <ul className="org-files">
        {files.slice(0, limit).map((f) => (
          <li key={f}>{f}</li>
        ))}
      </ul>
      {limit < files.length && (
        <button className="text-button" onClick={() => setLimit((n) => n + 40)}>
          Show more files ({files.length - limit} remaining)
        </button>
      )}
    </>
  );
}
function Members({
  members,
  onSelect,
}: {
  members: Component[];
  onSelect: (m: Component) => void;
}) {
  const [limit, setLimit] = useState(30);
  return (
    <div className="org-components">
      {members.slice(0, limit).map((m) => (
        <button key={refKey(m.reference)} onClick={() => onSelect(m)}>
          <span>
            <strong>{m.name}</strong>
            <small>{m.role}</small>
          </span>
          <Badge status={m.status} />
          <ChevronRight size={14} />
        </button>
      ))}
      {limit < members.length && (
        <button onClick={() => setLimit((n) => n + 30)}>
          Show more components ({members.length - limit} remaining)
        </button>
      )}
    </div>
  );
}
function ExpandableMembers({
  members,
  onSelect,
}: {
  members: Component[];
  onSelect: (m: Component) => void;
}) {
  const [open, setOpen] = useState(false);
  return (
    <details className="org-expand" onToggle={(e) => setOpen(e.currentTarget.open)}>
      <summary>Components ({members.length})</summary>
      {open && <Members members={members} onSelect={onSelect} />}
    </details>
  );
}
function Modal({
  title,
  onClose,
  children,
}: {
  title: string;
  onClose: () => void;
  children: React.ReactNode;
}) {
  const ref = useRef<HTMLElement>(null);
  useEffect(() => {
    const previous = document.activeElement as HTMLElement;
    const node = ref.current;
    const all = () => [
      ...(node?.querySelectorAll<HTMLElement>('button:not(:disabled),input,select,textarea') ?? []),
    ];
    all()[0]?.focus();
    const key = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
      if (e.key === 'Tab') {
        const elements = all();
        if (e.shiftKey && document.activeElement === elements[0]) {
          e.preventDefault();
          elements.at(-1)?.focus();
        } else if (!e.shiftKey && document.activeElement === elements.at(-1)) {
          e.preventDefault();
          elements[0]?.focus();
        }
      }
    };
    node?.addEventListener('keydown', key);
    return () => {
      node?.removeEventListener('keydown', key);
      if (previous?.isConnected) previous.focus();
    };
  }, []);
  return (
    <div className="modal-overlay org-modal">
      <section className="modal" ref={ref} role="dialog" aria-modal="true" aria-label={title}>
        <div className="modal-heading">
          <h2>{title}</h2>
          <button className="icon-button" aria-label="Close editor" onClick={onClose}>
            <X size={18} />
          </button>
        </div>
        {children}
      </section>
    </div>
  );
}
export default function OrganizationPage(props: Props) {
  const { mode, data, loading, busy, onRefresh } = props;
  const [query, setQuery] = useState('');
  const deferred = useDeferredValue(query);
  const [category, setCategory] = useState('all');
  const [status, setStatus] = useState('all');
  const [sort, setSort] = useState('name');
  const [layout, setLayout] = useState<'grid' | 'list'>('grid');
  const [page, setPage] = useState(0);
  const [selected, setSelected] = useState<string | null>(null);
  useEffect(() => {
    if (props.focusId) {
      setSelected(props.focusId);
      setComponent(null);
    }
  }, [props.focusId]);
  const [component, setComponent] = useState<Component | null>(null);
  const [pending, setPending] = useState(false);
  const [localError, setLocalError] = useState<string | null>(null);
  const [editor, setEditor] = useState<Collection | 'new' | null>(null);
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [editCategory, setEditCategory] = useState('Custom');
  const [members, setMembers] = useState<MemberRef[]>([]);
  const [memberQuery, setMemberQuery] = useState('');
  const [deleting, setDeleting] = useState<Collection | null>(null);
  const [editingSuit, setEditingSuit] = useState<Suit | null>(null);
  const [parentFilter, setParentFilter] = useState('all');
  const tooltipId = useId();
  const categoryHintId = (name: string) => `${tooltipId}-${name.replace(/[^a-z0-9]/gi, '-')}`;
  const [dismissedTooltip, setDismissedTooltip] = useState<string | null>(null);
  const [parentLimit, setParentLimit] = useState(20);
  const parentGroups = useMemo(() => {
    const groups = new Map<string, number>();
    for (const suit of data.suits) groups.set(suit.parent, (groups.get(suit.parent) ?? 0) + 1);
    return [...groups].sort(([a], [b]) => a.localeCompare(b));
  }, [data.suits]);
  function chooseParent(parent: string) {
    setParentFilter(parent);
    setComponent(null);
    setSelected(
      [...data.suits]
        .filter((s) => parent === 'all' || s.parent === parent)
        .sort((a, b) => a.name.localeCompare(b.name))[0]?.id ?? null,
    );
  }
  const [policeSection, setPoliceSection] = useState('all');
  const [relationshipConfirmed, setRelationshipConfirmed] = useState(false);
  const isSuit = mode === 'suits';
  const locked = busy || pending;
  const entries = isSuit ? data.suits : data.collections;
  useEffect(() => {
    if (entries.length && (!selected || !entries.some((item) => item.id === selected)))
      setSelected(
        entries.some((item) => item.id === props.focusId)
          ? props.focusId!
          : [...entries].sort((a, b) => a.name.localeCompare(b.name))[0].id,
      );
  }, [entries]);
  const categories = useMemo(
    () =>
      isSuit ? [...suitCategories] : [...new Set(data.collections.map((c) => c.category))].sort(),
    [data.collections, isSuit],
  );
  const filtered = useMemo(
    () =>
      entries
        .filter((item) => {
          const extra =
            'members' in item
              ? item.members.flatMap((m) => [m.name, ...m.files]).join(' ')
              : `${item.parent} ${item.model ?? ''} ${item.files.join(' ')}`;
          return (
            (category === 'all' ||
              item.category === category ||
              (category === 'Police & LSPDFR' &&
                'description' in item &&
                item.description.startsWith('RAGE Plugin Hook (shared dependency) · '))) &&
            (category !== 'Police & LSPDFR' ||
              policeSection === 'all' ||
              ('description' in item && item.description.startsWith(policeSection + ' · '))) &&
            (status === 'all' || item.status === status) &&
            (!isSuit ||
              parentFilter === 'all' ||
              ('parent' in item && item.parent === parentFilter)) &&
            `${item.name} ${extra}`.toLowerCase().includes(deferred.toLowerCase())
          );
        })
        .sort((a, b) =>
          sort === 'status'
            ? a.status.localeCompare(b.status) || a.name.localeCompare(b.name)
            : sort === 'category'
              ? a.category.localeCompare(b.category) || a.name.localeCompare(b.name)
              : sort === 'name-desc'
                ? b.name.localeCompare(a.name)
                : a.name.localeCompare(b.name),
        ),
    [entries, category, status, deferred, sort, parentFilter, isSuit, policeSection],
  );
  useEffect(
    () => setPage(0),
    [deferred, category, status, sort, mode, parentFilter, policeSection],
  );
  const maxPage = Math.max(0, Math.ceil(filtered.length / 24) - 1);
  const currentPage = Math.min(page, maxPage);
  const shown = filtered.slice(currentPage * 24, currentPage * 24 + 24);
  const selectedCollection = !isSuit ? data.collections.find((c) => c.id === selected) : undefined;
  const selectedSuit = isSuit ? data.suits.find((s) => s.id === selected) : undefined;
  const memberMatches = useMemo(
    () =>
      [
        ...data.available_members,
        ...(editor && editor !== 'new'
          ? editor.members.filter(
              (m) =>
                !data.available_members.some((a) => refKey(a.reference) === refKey(m.reference)),
            )
          : []),
      ]
        .filter((m) => `${m.name} ${m.location}`.toLowerCase().includes(memberQuery.toLowerCase()))
        .slice(0, 60),
    [data.available_members, memberQuery, editor],
  );
  async function mutate(fn: () => Promise<unknown>): Promise<boolean> {
    setPending(true);
    props.onBusy(true);
    setLocalError(null);
    try {
      await fn();
      await onRefresh();
      return true;
    } catch (e) {
      setLocalError(String(e));
      return false;
    } finally {
      setPending(false);
      props.onBusy(false);
    }
  }
  function edit(collection: Collection | 'new') {
    setEditor(collection);
    setName(collection === 'new' ? '' : collection.name);
    setDescription(collection === 'new' ? '' : collection.description);
    setEditCategory(collection === 'new' ? 'Custom' : collection.category);
    setMembers(collection === 'new' ? [] : collection.members.map((m) => m.reference));
    setRelationshipConfirmed(false);
    setMemberQuery('');
    setLocalError(null);
  }
  async function preview(kind: string, id: string, clear = false) {
    try {
      const path = clear ? null : await pick('preview');
      if (!clear && !path) return;
      await mutate(() => call('assign_preview', { kind, id, path }));
    } catch (e) {
      setLocalError(String(e));
    }
  }
  function selectMember(member: Component) {
    if (member.reference.kind === 'collection') {
      setSelected(member.reference.id);
      setComponent(null);
      return;
    }
    if (member.reference.kind === 'suit' && isSuit) {
      setSelected(member.reference.id);
      setComponent(null);
    } else setComponent(member);
  }
  const error = localError ?? props.error;
  return (
    <section
      className="organization-page"
      aria-label={isSuit ? 'Suits and skins manager' : 'Mod collections'}
    >
      <div className="org-heading">
        <div>
          <div className="eyebrow">{isSuit ? 'CHARACTER LIBRARY' : 'CONNECTED COMPONENTS'}</div>
          <h2>{isSuit ? 'Characters, suits & skins' : 'Your mod collections'}</h2>
          <p>
            {isSuit
              ? 'Identifiable outfits grouped by their declared character or package. Uncertain resources stay in the Advanced File Inspector.'
              : 'Packages stay together. Custom collections reference your existing records.'}
          </p>
        </div>
        <div className="button-row">
          {!isSuit && (
            <button className="button primary" disabled={locked} onClick={() => edit('new')}>
              <Plus size={15} />
              New collection
            </button>
          )}
          <button className="button" disabled={locked || loading} onClick={() => void onRefresh()}>
            {loading ? <LoaderCircle size={15} className="spin" /> : <Boxes size={15} />}Refresh
          </button>
        </div>
      </div>
      {isSuit && (
        <div className="org-note">
          <ShieldCheck size={17} />
          <p>
            Enable and disable change an approved installation package. In-game suit selection and
            equipping are not available here. Archive resources remain read-only when their
            ownership is shared or unknown.
          </p>
        </div>
      )}
      {error && (
        <div className="alert danger" role="alert">
          <TriangleAlert size={17} />
          <p>{error}</p>
        </div>
      )}
      {data.warnings.length > 0 && (
        <details className="org-warnings">
          <summary>
            <TriangleAlert size={14} />
            Detection notes ({data.warnings.length})
          </summary>
          {data.warnings.map((w, i) => (
            <p key={i}>{w}</p>
          ))}
        </details>
      )}
      <div className="neon-category-rail" aria-label="Quick categories">
        {(isSuit
          ? [
              { name: 'all', label: 'All outfits', icon: Grid2X2 },
              { name: 'Superhero Suits', label: 'Superhero Suits', icon: Sparkles },
              { name: 'Police Uniforms', label: 'Police Uniforms', icon: Shield },
              { name: 'Custom Peds', label: 'Custom Peds', icon: Users },
              { name: 'Civilian Outfits', label: 'Civilian Outfits', icon: Shirt },
            ]
          : [
              { name: 'all', label: 'All collections', icon: Grid2X2 },
              { name: 'Vehicles', label: 'Vehicles', icon: Car },
              { name: 'Police & LSPDFR', label: 'Police & LSPDFR', icon: Shield },
              { name: 'Superhero Mods', label: 'Superhero Mods', icon: Sparkles },
              { name: 'Peds & Skins', label: 'Peds & Skins', icon: Users },
            ]
        ).map((item) => (
          <button
            key={item.name}
            title={item.label}
            aria-label={`Show ${item.label}`}
            aria-describedby={categoryHintId(item.name)}
            onFocus={() => setDismissedTooltip(null)}
            onMouseEnter={() => setDismissedTooltip(null)}
            onKeyDown={(e) => {
              if (e.key === 'Escape') {
                e.stopPropagation();
                setDismissedTooltip(item.name);
              }
            }}
            aria-pressed={category === item.name}
            onClick={() => setCategory(item.name)}
          >
            <item.icon size={23} aria-hidden="true" />
            <span
              className="category-tooltip"
              role="tooltip"
              hidden={dismissedTooltip === item.name}
              id={categoryHintId(item.name)}
            >
              {item.label}
            </span>
          </button>
        ))}
      </div>
      <p className="selected-category" aria-live="polite">
        Category:{' '}
        <strong>
          {category === 'all' ? (isSuit ? 'All outfits' : 'All collections') : category}
        </strong>
      </p>
      {isSuit && (
        <div className="character-browser" role="group" aria-label="Characters and parent packages">
          <strong>Characters & packages</strong>
          <div className="character-choices">
            <button aria-pressed={parentFilter === 'all'} onClick={() => chooseParent('all')}>
              All outfits <small>{data.suits.length}</small>
            </button>
            {parentGroups.slice(0, parentLimit).map(([parent, count]) => (
              <button
                key={parent}
                aria-label={`Browse ${parent}`}
                aria-pressed={parentFilter === parent}
                onClick={() => chooseParent(parent)}
              >
                {parent}
                <small>
                  {count} {count === 1 ? 'outfit' : 'outfits'}
                </small>
              </button>
            ))}
            {parentGroups.length > parentLimit && (
              <button onClick={() => setParentLimit((n) => n + 20)}>
                More characters & packages
              </button>
            )}
          </div>
        </div>
      )}
      <div className="org-filters">
        {!isSuit && category === 'Police & LSPDFR' && (
          <select
            aria-label="Police package section"
            value={policeSection}
            onChange={(e) => setPoliceSection(e.target.value)}
          >
            <option value="all">All police packages</option>
            {[
              'LSPDFR Core',
              'RAGE Plugin Hook (shared dependency)',
              'Police Callout Packs',
              'Police Plugins',
              'Police Vehicle Packs',
              'Police Uniforms & Peds',
              'Police Audio and Other Add-ons',
            ].map((section) => (
              <option key={section}>{section}</option>
            ))}
          </select>
        )}
        {isSuit && (
          <select
            aria-label="Parent character or package"
            value={parentFilter}
            onChange={(e) => chooseParent(e.target.value)}
          >
            <option value="all">All parent packages</option>
            {[...new Set(data.suits.map((s) => s.parent))].sort().map((parent) => (
              <option key={parent}>{parent}</option>
            ))}
          </select>
        )}
        <label className="search-box">
          <Search size={16} />
          <input
            aria-label={isSuit ? 'Search suits' : 'Search collections'}
            placeholder={
              isSuit
                ? 'Search suits, models or file locations…'
                : 'Search collections and components…'
            }
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
        </label>
        <select
          aria-label={isSuit ? 'Suit category' : 'Collection category'}
          value={category}
          onChange={(e) => setCategory(e.target.value)}
        >
          <option value="all">All categories</option>
          {categories.map((c) => (
            <option key={c}>{c}</option>
          ))}
        </select>
        <select
          aria-label="Organization status"
          value={status}
          onChange={(e) => setStatus(e.target.value)}
        >
          <option value="all">All statuses</option>
          {['enabled', 'disabled', 'partial', 'unknown', 'needs_review', 'imported', 'empty'].map(
            (s) => (
              <option value={s} key={s}>
                {label(s)}
              </option>
            ),
          )}
        </select>
        <select
          aria-label="Organization sorting"
          value={sort}
          onChange={(e) => setSort(e.target.value)}
        >
          <option value="name">Name A–Z</option>
          <option value="name-desc">Name Z–A</option>
          <option value="category">Category</option>
          <option value="status">Status</option>
        </select>
        <div className="org-view-switch">
          <button
            className="icon-button"
            aria-label="Grid view"
            aria-pressed={layout === 'grid'}
            onClick={() => setLayout('grid')}
          >
            <Grid2X2 size={17} />
          </button>
          <button
            className="icon-button"
            aria-label="List view"
            aria-pressed={layout === 'list'}
            onClick={() => setLayout('list')}
          >
            <List size={17} />
          </button>
        </div>
      </div>
      <div className="org-result-count" role="status">
        {loading
          ? 'Refreshing metadata…'
          : `${filtered.length} ${isSuit ? 'identified outfits' : 'collections'} · ${entries.length} total`}
      </div>
      <div
        className={`org-workspace ${selectedCollection || selectedSuit || component ? 'with-details' : ''}`}
      >
        <div>
          <div className={`org-items ${layout}`}>
            {shown.map((item) => (
              <article
                className={`org-card ${item.id === selected ? 'selected' : ''}`}
                key={item.id}
              >
                <button
                  className="org-card-main"
                  aria-label={`View ${isSuit ? 'suit' : 'collection'} ${item.name}`}
                  onClick={() => {
                    setSelected(item.id);
                    setComponent(null);
                  }}
                >
                  <Thumbnail name={item.name} preview={item.preview} category={item.category} />
                  <span className="card-layer-icon" aria-hidden="true">
                    <Boxes size={18} />
                  </span>
                  <div className="org-card-copy">
                    <span className="evidence-caption">
                      {'members' in item
                        ? item.automatic
                          ? 'Detected package / relationship'
                          : 'User-selected collection'
                        : 'Identified outfit · compatibility unverified'}
                    </span>
                    <Badge status={item.status} />
                    <h3>{item.name}</h3>
                    <p>{item.category}</p>
                    {'members' in item ? (
                      <small>
                        {item.unique_files} unique files · {item.members.length} related components
                      </small>
                    ) : (
                      <small>
                        {item.parent} · {item.source}
                      </small>
                    )}
                    <span className="org-relationship">
                      {'automatic' in item
                        ? item.automatic
                          ? 'Automatic relationship'
                          : 'Custom collection'
                        : item.manageable
                          ? 'Independent package'
                          : 'Review required'}
                    </span>
                  </div>
                </button>
                {'members' in item && (
                  <ExpandableMembers members={item.members} onSelect={selectMember} />
                )}
                {'parent' in item && <OutfitFiles files={item.files} />}
              </article>
            ))}
          </div>
          {!shown.length && (
            <div className="org-empty">
              <Boxes size={34} />
              <h3>
                {query || category !== 'all' || status !== 'all'
                  ? 'No matching items'
                  : isSuit
                    ? 'No suits detected yet'
                    : 'Your collections start here'}
              </h3>
              <p>
                {isSuit
                  ? 'Scan a synthetic or user-selected Legacy installation, or import a package with explicit suit metadata. Unknown archive contents are not invented as individual suits.'
                  : 'Import a mod to see its package grouping, or create a custom collection from your existing records.'}
              </p>
            </div>
          )}
          {filtered.length > 24 && (
            <div className="org-pagination">
              <button
                className="button"
                disabled={currentPage === 0}
                onClick={() => setPage((p) => Math.max(0, p - 1))}
              >
                Previous
              </button>
              <span>
                Page {currentPage + 1} of {maxPage + 1}
              </span>
              <button
                className="button"
                disabled={currentPage === maxPage}
                onClick={() => setPage((p) => p + 1)}
              >
                Next
              </button>
            </div>
          )}
        </div>
        {(selectedCollection || selectedSuit || component) && (
          <aside className="org-detail" aria-label="Organization details">
            <div className="org-detail-title">
              <span className="eyebrow">DETAILS</span>
              <button
                className="icon-button"
                aria-label="Close details"
                onClick={() => {
                  setSelected(null);
                  setComponent(null);
                }}
              >
                <X size={17} />
              </button>
            </div>
            {component ? (
              <>
                <h3>{component.name}</h3>
                <Badge status={component.status} />
                <p>{component.role}</p>
                <p>{component.control_reason}</p>
                <h4>File locations</h4>
                <code className="path">{component.location}</code>
                <FileGroups files={component.files} />
                <Files files={component.files} />
                {component.mod_id && (
                  <button className="button" onClick={() => props.onParent(component.mod_id!)}>
                    View parent mod
                  </button>
                )}
              </>
            ) : selectedSuit ? (
              <>
                <Thumbnail
                  name={selectedSuit.name}
                  preview={selectedSuit.preview}
                  category={selectedSuit.category}
                />
                <h3>{selectedSuit.name}</h3>
                <Badge status={selectedSuit.status} />
                <dl>
                  <dt>Parent package</dt>
                  <dd>{selectedSuit.parent}</dd>
                  <dt>Character / model</dt>
                  <dd>{selectedSuit.model ?? 'Unknown — not inferred from filenames'}</dd>
                  <dt>Source</dt>
                  <dd>{selectedSuit.source}</dd>
                  <dt>Category</dt>
                  <dd>{selectedSuit.category}</dd>
                </dl>
                <h4>Collections</h4>
                <p>
                  {data.collections
                    .filter((c) =>
                      c.members.some(
                        (m) => m.reference.kind === 'suit' && m.reference.id === selectedSuit.id,
                      ),
                    )
                    .map((c) => c.name)
                    .join(', ') || 'No collection membership'}
                </p>
                <h4>Dependencies</h4>
                <p>{selectedSuit.dependencies.join(', ') || 'Not established'}</p>
                {selectedSuit.conflicts.map((c) => (
                  <p className="inline-warning" key={c}>
                    {c}
                  </p>
                ))}
                <h4>Management</h4>
                <p
                  id="suit-management-reason"
                  className={selectedSuit.manageable ? '' : 'inline-warning'}
                >
                  {locked
                    ? 'Another operation is active. Wait for it to finish before reviewing a suit transaction. '
                    : selectedSuit.manageable &&
                        !['enabled', 'disabled'].includes(selectedSuit.status)
                      ? 'The parent package is not currently installed in an enabled or disabled state. Review its installation first. '
                      : ''}
                  {selectedSuit.control_reason}
                </p>
                <button
                  className="button primary"
                  aria-describedby="suit-management-reason"
                  disabled={
                    locked ||
                    !selectedSuit.manageable ||
                    !['enabled', 'disabled'].includes(selectedSuit.status)
                  }
                  onClick={() =>
                    props.onReviewSuit(
                      selectedSuit.id,
                      selectedSuit.status === 'disabled' ? 'enable' : 'disable',
                    )
                  }
                >
                  Review {selectedSuit.status === 'disabled' ? 'enable' : 'disable'} suit
                </button>
                <h4>File locations</h4>
                <FileGroups files={selectedSuit.files} />
                {selectedSuit.notes.map((n) => (
                  <p className="subtle" key={n}>
                    {n}
                  </p>
                ))}
                <div className="button-row">
                  <button
                    className="button"
                    disabled={locked}
                    onClick={() => {
                      setEditingSuit(selectedSuit);
                      setName(selectedSuit.name);
                      setEditCategory(selectedSuit.category);
                    }}
                  >
                    <Pencil size={14} />
                    Edit label/category
                  </button>
                  {selectedSuit.mod_id && (
                    <button className="button" onClick={() => props.onParent(selectedSuit.mod_id!)}>
                      Parent mod
                    </button>
                  )}
                </div>
                <div className="button-row">
                  <button
                    className="button"
                    disabled={locked}
                    onClick={() => void preview('suit', selectedSuit.id)}
                  >
                    <ImagePlus size={14} />
                    Assign preview
                  </button>
                  {selectedSuit.preview && (
                    <button
                      className="text-button"
                      disabled={locked}
                      onClick={() => void preview('suit', selectedSuit.id, true)}
                    >
                      Clear preview
                    </button>
                  )}
                </div>
              </>
            ) : (
              selectedCollection && (
                <>
                  <Thumbnail
                    name={selectedCollection.name}
                    preview={selectedCollection.preview}
                    category={selectedCollection.category}
                  />
                  <h3>{selectedCollection.name}</h3>
                  <Badge status={selectedCollection.status} />
                  <p>{selectedCollection.description}</p>
                  <h4>Why these are grouped</h4>
                  <p>{selectedCollection.relationship}</p>
                  <p className="inline-warning">
                    This collection has no bulk toggle. Select an individually supported component
                    to review its operation plan. Inferred grouping never establishes ownership or
                    permission to change shared files.
                  </p>
                  <p>
                    {selectedCollection.unique_files} unique files ·{' '}
                    {selectedCollection.members.length} component references
                  </p>
                  <div className="org-status-counts">
                    {Object.entries(
                      selectedCollection.members.reduce<Record<string, number>>(
                        (all, m) => ({ ...all, [m.status]: (all[m.status] ?? 0) + 1 }),
                        {},
                      ),
                    ).map(([s, count]) => (
                      <span key={s}>
                        {count} {label(s)}
                      </span>
                    ))}
                  </div>
                  <h4>Components</h4>
                  <Members members={selectedCollection.members} onSelect={selectMember} />
                  <h4>Associated file groups</h4>
                  <FileGroups files={selectedCollection.members.flatMap((m) => m.files)} />
                  <div className="button-row">
                    {selectedCollection.automatic && (
                      <button
                        className="button"
                        disabled={locked}
                        onClick={() => edit(selectedCollection)}
                      >
                        <Pencil size={14} />
                        Correct virtual folder
                      </button>
                    )}
                    {!selectedCollection.automatic && (
                      <>
                        <button
                          className="button"
                          disabled={locked}
                          onClick={() => edit(selectedCollection)}
                        >
                          <Pencil size={14} />
                          Edit collection
                        </button>
                        <button
                          className="button danger"
                          disabled={locked}
                          onClick={() => setDeleting(selectedCollection)}
                        >
                          <Trash2 size={14} />
                          Remove collection
                        </button>
                      </>
                    )}
                    <button
                      className="button"
                      disabled={locked}
                      onClick={() => void preview('collection', selectedCollection.id)}
                    >
                      <ImagePlus size={14} />
                      Assign preview
                    </button>
                    {selectedCollection.preview && (
                      <button
                        className="text-button"
                        disabled={locked}
                        onClick={() => void preview('collection', selectedCollection.id, true)}
                      >
                        Clear preview
                      </button>
                    )}
                  </div>
                </>
              )
            )}
          </aside>
        )}
      </div>
      {editor && (
        <Modal
          title={editor === 'new' ? 'Create collection' : 'Edit collection'}
          onClose={() => {
            if (!locked) setEditor(null);
          }}
        >
          <form
            onSubmit={(e) => {
              e.preventDefault();
              void mutate(() =>
                call(
                  editor !== 'new' && editor.automatic ? 'save_virtual_group' : 'save_collection',
                  {
                    id: editor === 'new' ? null : editor.id,
                    name,
                    description,
                    category: editCategory,
                    members,
                    ...(editor !== 'new' && editor.automatic
                      ? { confirmed: relationshipConfirmed }
                      : {}),
                  },
                ),
              ).then((ok) => {
                if (ok) setEditor(null);
              });
            }}
          >
            <div className="modal-body org-editor">
              <label>
                Collection name
                <input
                  required
                  maxLength={120}
                  aria-label="Collection name"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                />
              </label>
              <label>
                Category
                <input
                  required
                  maxLength={120}
                  aria-label="Collection editor category"
                  value={editCategory}
                  onChange={(e) => setEditCategory(e.target.value)}
                />
              </label>
              <label>
                Description
                <textarea
                  maxLength={2000}
                  aria-label="Collection description"
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                />
              </label>
              <p>
                Membership links reference existing records. Files, ownership and backups remain
                with their original mods.
              </p>
              {editor !== 'new' && editor.automatic && (
                <label className="approval-inline">
                  <input
                    type="checkbox"
                    checked={relationshipConfirmed}
                    onChange={(e) => setRelationshipConfirmed(e.target.checked)}
                  />
                  I confirm these visual relationships from package documentation. This does not
                  confirm compatibility or grant file-management authority.
                </label>
              )}
              <label>
                Find components
                <input
                  aria-label="Find collection components"
                  value={memberQuery}
                  onChange={(e) => setMemberQuery(e.target.value)}
                />
              </label>
              <p>{members.length} selected · up to 60 search results shown</p>
              <div className="org-member-picker">
                {memberMatches.map((m) => (
                  <label key={refKey(m.reference)}>
                    <input
                      type="checkbox"
                      aria-label={
                        m.status === 'missing'
                          ? `Include missing member ${m.reference.id}`
                          : undefined
                      }
                      checked={members.some((r) => refKey(r) === refKey(m.reference))}
                      onChange={(e) =>
                        setMembers((previous) =>
                          e.target.checked
                            ? [...previous, m.reference]
                            : previous.filter((r) => refKey(r) !== refKey(m.reference)),
                        )
                      }
                    />
                    <span>
                      {m.name}
                      <small>
                        {m.status === 'missing'
                          ? `${m.role} · ${m.reference.kind} ${m.reference.id}`
                          : `${m.role} · ${m.source}`}
                      </small>
                    </span>
                  </label>
                ))}
              </div>
              {error && (
                <p className="inline-warning" role="alert">
                  {error}
                </p>
              )}
            </div>
            <div className="modal-footer">
              <button
                type="button"
                className="button"
                disabled={locked}
                onClick={() => setEditor(null)}
              >
                Cancel
              </button>
              <span className="spacer" />
              <button className="button primary" disabled={locked || !name.trim()}>
                Save collection
              </button>
            </div>
          </form>
        </Modal>
      )}
      {deleting && (
        <Modal
          title="Remove collection"
          onClose={() => {
            if (!locked) setDeleting(null);
          }}
        >
          <div className="modal-body">
            <p>
              Remove “{deleting.name}” and its membership links? Managed mods, game files and
              retained backups stay in place.
            </p>
          </div>
          <div className="modal-footer">
            <button className="button" disabled={locked} onClick={() => setDeleting(null)}>
              Cancel
            </button>
            <button
              className="button danger"
              disabled={locked}
              onClick={() =>
                void mutate(() => call('remove_collection', { id: deleting.id })).then((ok) => {
                  if (ok) {
                    setDeleting(null);
                    setSelected(null);
                  }
                })
              }
            >
              Remove collection
            </button>
          </div>
        </Modal>
      )}
      {editingSuit && (
        <Modal
          title="Edit suit details"
          onClose={() => {
            if (!locked) setEditingSuit(null);
          }}
        >
          <form
            onSubmit={(e) => {
              e.preventDefault();
              void mutate(() =>
                call('rename_suit', { id: editingSuit.id, name, category: editCategory }),
              ).then((ok) => {
                if (ok) setEditingSuit(null);
              });
            }}
          >
            <div className="modal-body org-editor">
              <label>
                Suit name
                <input
                  required
                  maxLength={120}
                  aria-label="Suit name"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                />
              </label>
              <label>
                Suit category
                <select
                  aria-label="Suit editor category"
                  value={editCategory}
                  onChange={(e) => setEditCategory(e.target.value)}
                >
                  {suitCategories.map((c) => (
                    <option key={c}>{c}</option>
                  ))}
                </select>
              </label>
              <p>
                Labels are your metadata. Changing a label or category does not establish model
                ownership or unlock unsafe controls.
              </p>
            </div>
            <div className="modal-footer">
              <button
                className="button"
                type="button"
                disabled={locked}
                onClick={() => setEditingSuit(null)}
              >
                Cancel
              </button>
              <button className="button primary" disabled={locked}>
                Save suit details
              </button>
            </div>
          </form>
        </Modal>
      )}
    </section>
  );
}
