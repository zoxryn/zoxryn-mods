import { useCallback, useEffect, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { browserPreview, call } from './api';
import { version } from '../package.json';
export interface AppUpdateStatus {
  installed_version: string;
  configured: boolean;
  phase: string;
  available_version: string | null;
  release_notes: string;
  size_bytes: number | null;
  downloaded_bytes: number;
  total_bytes: number | null;
  error: string | null;
  can_install: boolean;
  eligibility_message: string;
  approval_token: string | null;
  metadata_verified: boolean;
  last_checked_at: number | null;
}
const initial: AppUpdateStatus = {
  installed_version: version,
  configured: false,
  phase: 'idle',
  available_version: null,
  release_notes: '',
  size_bytes: null,
  downloaded_bytes: 0,
  total_bytes: null,
  error: null,
  can_install: false,
  eligibility_message: 'Checking update configuration…',
  approval_token: null,
  metadata_verified: false,
  last_checked_at: null,
};
export function useAppUpdates() {
  const [status, setStatus] = useState(initial);
  const [prompt, setPrompt] = useState(false);
  const [deferred, setDeferred] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const started = useRef(false);
  const check = useCallback(async () => {
    setError(null);
    setDeferred(false);
    if (browserPreview) {
      setStatus({
        ...initial,
        phase: 'not_configured',
        eligibility_message: 'Updates are available in the installed Windows application.',
      });
      return;
    }
    try {
      const next = await call<AppUpdateStatus>('check_app_update');
      setStatus(next);
      if (next.phase === 'available') setPrompt(true);
    } catch (e) {
      setError(String(e));
      try {
        setStatus(await call<AppUpdateStatus>('update_status'));
      } catch {
        /* Compatibility with an unavailable native bridge. */
      }
    }
  }, []);
  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;
    if (!browserPreview)
      void listen<AppUpdateStatus>('app-update-status', (event) => {
        if (active) setStatus(event.payload);
      })
        .then((stop) => {
          if (active) unlisten = stop;
          else stop();
        })
        .catch(() => {});
    if (!started.current) {
      started.current = true;
      void check();
    }
    return () => {
      active = false;
      unlisten?.();
    };
  }, [check]);
  const install = useCallback(async () => {
    if (!status.approval_token) return;
    setError(null);
    setStatus((previous) => ({ ...previous, phase: 'downloading', downloaded_bytes: 0 }));
    try {
      await call('approve_app_update', { token: status.approval_token, approved: true });
    } catch (e) {
      setError(String(e));
      try {
        setStatus(await call<AppUpdateStatus>('update_status'));
      } catch {
        setStatus((previous) => ({ ...previous, phase: 'error' }));
      }
    }
  }, [status.approval_token]);
  return {
    status,
    error,
    prompt,
    setPrompt,
    deferred,
    later: () => {
      setDeferred(true);
      setPrompt(false);
    },
    check,
    install,
    active: ['downloading', 'verifying', 'installing'].includes(status.phase),
  };
}
