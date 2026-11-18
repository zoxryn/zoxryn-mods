import { useEffect, useRef } from 'react';
import {
  ArrowDownToLine,
  Check,
  CheckCheck,
  Download,
  ExternalLink,
  LoaderCircle,
  RefreshCw,
  ShieldCheck,
  TriangleAlert,
  X,
} from 'lucide-react';
import { call } from './api';
import './AppUpdates.css';
import type { useAppUpdates } from './useAppUpdates';
type Updates = ReturnType<typeof useAppUpdates>;
const bytes = (n: number | null) =>
  n === null
    ? 'Size unknown'
    : n >= 1048576
      ? `${(n / 1048576).toFixed(1)} MB`
      : `${Math.ceil(n / 1024)} KB`;
function Controls({ updates, fileBusy }: { updates: Updates; fileBusy: boolean }) {
  const { status, active, check, install } = updates;
  const canApprove = !!status.approval_token && status.can_install && !active && !fileBusy;
  return (
    <div className="update-controls">
      <button
        className="button"
        disabled={active || status.phase === 'checking'}
        onClick={() => void check()}
      >
        {status.phase === 'checking' ? (
          <LoaderCircle className="spin" size={15} />
        ) : (
          <RefreshCw size={15} />
        )}
        Check for Updates
      </button>
      {status.available_version && (
        <button className="button primary" disabled={!canApprove} onClick={() => void install()}>
          {active ? <LoaderCircle className="spin" size={15} /> : <ArrowDownToLine size={15} />}
          Update & Restart
        </button>
      )}
      {status.available_version && !active && !updates.deferred && (
        <button className="button" onClick={updates.later}>
          Later
        </button>
      )}
    </div>
  );
}
function Progress({ updates }: { updates: Updates }) {
  const { status, active } = updates;
  if (!active) return null;
  const percent = status.total_bytes
    ? Math.min(100, Math.round((status.downloaded_bytes / status.total_bytes) * 100))
    : null;
  return (
    <div className="update-progress" role="status">
      <div>
        <strong>
          {status.phase === 'verifying'
            ? 'Verifying cryptographic signatures'
            : status.phase === 'installing'
              ? 'Installing the update and restarting…'
              : 'Downloading your approved update'}
        </strong>
        <span>
          {status.phase === 'downloading'
            ? `${bytes(status.downloaded_bytes)}${status.total_bytes ? ` / ${bytes(status.total_bytes)}` : ''}`
            : ''}
        </span>
      </div>
      <progress
        aria-label="Application update progress"
        max={100}
        {...(percent !== null ? { value: percent } : {})}
      />
      <p>
        Your mod files are not part of this update. New file operations are blocked until this
        attempt finishes.
      </p>
    </div>
  );
}
export function UpdateSettings({ updates, fileBusy }: { updates: Updates; fileBusy: boolean }) {
  const { status, error, active } = updates;
  return (
    <section className="update-card">
      <div className="update-title">
        <span className="settings-icon">
          <Download size={25} />
        </span>
        <div>
          <h2>Application updates</h2>
          <p>Signed Windows updates, installed after your approval.</p>
        </div>
        <span className="badge imported">Installed v{status.installed_version}</span>
      </div>
      <div className="update-version-grid">
        <div>
          <span>Installed version</span>
          <strong>{status.installed_version}</strong>
        </div>
        <div>
          <span>Available version</span>
          <strong>
            {status.available_version ??
              (status.phase === 'up_to_date' ? 'You’re up to date' : 'Not announced')}
          </strong>
        </div>
        <div>
          <span>Update size</span>
          <strong>{bytes(status.size_bytes)}</strong>
        </div>
        <div>
          <span>Last update check</span>
          <strong>
            {status.last_checked_at
              ? new Date(status.last_checked_at * 1000).toLocaleString()
              : 'Never checked'}
          </strong>
        </div>
        <div>
          <span>Update status</span>
          <strong>
            {(
              {
                idle: 'Ready to check',
                checking: 'Checking',
                available: 'Update available',
                up_to_date: 'Up to date',
                not_published: 'No release published',
                error: 'Check failed',
                downloading: 'Downloading',
                verifying: 'Verifying',
                installing: 'Installing',
                not_configured: 'Not configured',
              } as Record<string, string>
            )[status.phase] || status.phase}
          </strong>
        </div>
      </div>
      {status.phase === 'not_published' && (
        <p className="subtle">
          No stable signed release is published yet. Zoxryn Mods remains usable; check again after a
          release is approved and published.
        </p>
      )}
      {status.metadata_verified && (
        <p className="update-trust">
          <ShieldCheck size={14} />
          Release manifest verified. Installer signatures and version will be verified before
          installation.
        </p>
      )}
      {!status.configured && (
        <div className="alert">
          <TriangleAlert size={17} />
          <p>
            The release feed is not configured yet. No update server is contacted and nothing is
            downloaded. A GitHub release repository must be configured before online updates become
            available.
          </p>
        </div>
      )}
      {status.phase === 'up_to_date' && (
        <p className="update-trust">
          <CheckCheck size={15} />
          No newer signed release is available.
        </p>
      )}
      {status.available_version && (
        <div className="release-notes">
          <h3>Release notes · {status.available_version}</h3>
          <pre>{status.release_notes || 'No release notes provided.'}</pre>
        </div>
      )}
      {(error || status.error) && (
        <div className="alert danger" role="alert">
          <TriangleAlert size={17} />
          <p>{error ?? status.error}</p>
        </div>
      )}
      <p className="subtle">{status.eligibility_message}</p>
      {fileBusy && !active && (
        <p className="inline-warning">
          <TriangleAlert size={14} />
          Finish the current scan or mod/file operation before updating.
        </p>
      )}
      <Progress updates={updates} />
      {updates.deferred && status.available_version && (
        <p className="subtle" role="status">
          Update deferred. Continue using Zoxryn Mods and select Update &amp; Restart when you are
          ready.
        </p>
      )}
      <Controls updates={updates} fileBusy={fileBusy} />
      <div className="update-recovery">
        <strong>Need manual recovery?</strong>
        <p>
          Use the supplied recovery installer to repair the application. Keep app data and backups;
          never remove them to fix an update.
        </p>
        <button
          className="text-button"
          disabled={!status.configured || active}
          onClick={() => void call('open_recovery_page').catch(() => {})}
        >
          Open official release page
          <ExternalLink size={13} />
        </button>
      </div>
    </section>
  );
}
export function UpdatePrompt({ updates, fileBusy }: { updates: Updates; fileBusy: boolean }) {
  const dialog = useRef<HTMLElement>(null);
  const previous = useRef<HTMLElement | null>(null);
  const { status, active } = updates;
  useEffect(() => {
    if (!updates.prompt) return;
    previous.current = document.activeElement as HTMLElement;
    const element = dialog.current;
    const controls = () => [
      ...(element?.querySelectorAll<HTMLElement>('button:not(:disabled)') ?? []),
    ];
    controls()[0]?.focus();
    const key = (event: KeyboardEvent) => {
      if (event.key === 'Escape' && !active) updates.setPrompt(false);
      if (event.key === 'Tab') {
        const all = controls();
        if (event.shiftKey && document.activeElement === all[0]) {
          event.preventDefault();
          all.at(-1)?.focus();
        } else if (!event.shiftKey && document.activeElement === all.at(-1)) {
          event.preventDefault();
          all[0]?.focus();
        }
      }
    };
    element?.addEventListener('keydown', key);
    return () => {
      element?.removeEventListener('keydown', key);
      if (previous.current?.isConnected) previous.current.focus();
    };
  }, [updates.prompt, active]);
  if (!updates.prompt) return null;
  return (
    <div className="modal-overlay update-overlay">
      <section
        ref={dialog}
        className="modal"
        role="dialog"
        aria-modal="true"
        aria-label="Application update available"
      >
        <div className="modal-heading">
          <div>
            <div className="eyebrow">VERIFIED APPLICATION RELEASE</div>
            <h2>Zoxryn Mods {status.available_version}</h2>
          </div>
          <button
            className="icon-button"
            aria-label="Dismiss update for later"
            disabled={active}
            onClick={() => updates.setPrompt(false)}
          >
            <X size={18} />
          </button>
        </div>
        <div className="modal-body">
          <div className="detail-badges">
            <span className="badge disabled">Installed {status.installed_version}</span>
            <span className="badge enabled">Available {status.available_version}</span>
            <span className="badge imported">{bytes(status.size_bytes)}</span>
          </div>
          <div className="release-notes">
            <h3>What’s new</h3>
            <pre>{status.release_notes || 'No release notes provided.'}</pre>
          </div>
          <p className="subtle">
            Update & Restart downloads and verifies this approved Windows update, preserves your
            records and backups, then restarts Zoxryn Mods. GTA V files are not updated.
          </p>
          {!status.can_install && (
            <p className="inline-warning">
              <TriangleAlert size={14} />
              {status.eligibility_message}
            </p>
          )}
          {fileBusy && !active && (
            <p className="inline-warning">
              <TriangleAlert size={14} />
              Finish the active file operation before updating.
            </p>
          )}
          {(updates.error || status.error) && (
            <div className="alert danger" role="alert">
              <TriangleAlert size={17} />
              <p>{updates.error ?? status.error}</p>
            </div>
          )}
          <Progress updates={updates} />
        </div>
        <div className="modal-footer">
          <button className="button" disabled={active} onClick={updates.later}>
            Later
          </button>
          <span className="spacer" />
          <button
            className="button primary"
            disabled={active || fileBusy || !status.can_install || !status.approval_token}
            onClick={() => void updates.install()}
          >
            {active ? <LoaderCircle className="spin" size={15} /> : <Download size={15} />}Update &
            Restart
          </button>
        </div>
      </section>
    </div>
  );
}
