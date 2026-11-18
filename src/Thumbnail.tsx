import { useEffect, useRef, useState } from 'react';
import { Image as ImageIcon } from 'lucide-react';
import { call } from './api';
const cache = new Map<string, string>();
const inFlight = new Map<string, Promise<string>>();
const queued: Array<() => void> = [];
let active = 0;
function thumbnail(key: string): Promise<string> {
  const cached = cache.get(key);
  if (cached) {
    cache.delete(key);
    cache.set(key, cached);
    return Promise.resolve(cached);
  }
  const current = inFlight.get(key);
  if (current) return current;
  const result = new Promise<string>((resolve, reject) => {
    queued.push(() => {
      active++;
      void call<string>('read_preview', { key })
        .then((value) => {
          cache.set(key, value);
          while (cache.size > 16) cache.delete(cache.keys().next().value!);
          resolve(value);
        }, reject)
        .finally(() => {
          active--;
          inFlight.delete(key);
          pump();
        });
    });
    pump();
  });
  inFlight.set(key, result);
  return result;
}
function pump() {
  while (active < 3 && queued.length) queued.shift()!();
}
export default function Thumbnail({ name, preview }: { name: string; preview: string | null }) {
  const element = useRef<HTMLDivElement>(null);
  const [loaded, setLoaded] = useState<{ key: string; source: string } | null>(null);
  const source = loaded?.key === preview ? loaded.source : null;
  useEffect(() => {
    let alive = true;
    setLoaded(null);
    if (!preview) return;
    let started = false;
    const load = () => {
      if (started) return;
      started = true;
      void thumbnail(preview)
        .then((image) => {
          if (alive) setLoaded({ key: preview, source: image });
        })
        .catch(() => {});
    };
    if (typeof IntersectionObserver === 'undefined') load();
    const observer =
      typeof IntersectionObserver === 'undefined'
        ? null
        : new IntersectionObserver(
            (entries) => {
              if (entries.some((e) => e.isIntersecting)) {
                load();
                observer?.disconnect();
              }
            },
            { rootMargin: '80px' },
          );
    if (element.current) observer?.observe(element.current);
    return () => {
      alive = false;
      observer?.disconnect();
    };
  }, [preview]);
  return (
    <div className="org-thumbnail" ref={element}>
      {source ? (
        <img
          src={source}
          alt={`${name} — assigned preview`}
          loading="lazy"
          decoding="async"
          onError={() => setLoaded(null)}
        />
      ) : (
        <div className="org-placeholder">
          <ImageIcon size={27} />
          <span>{name}</span>
          <small>{preview ? 'Preview loading or unavailable' : 'No preview assigned'}</small>
        </div>
      )}
    </div>
  );
}
