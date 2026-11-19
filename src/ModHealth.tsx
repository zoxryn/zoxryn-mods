import { ShieldCheck, RefreshCw, TriangleAlert } from 'lucide-react';
import type { HealthReport } from './organizationTypes';
export default function ModHealth({
  health,
  busy,
  onCheck,
}: {
  health?: HealthReport;
  busy: boolean;
  onCheck: () => void;
}) {
  return (
    <section className="panel mod-health" aria-label="Mod Health Check">
      <div className="section-heading">
        <h2>
          <ShieldCheck size={18} />
          Mod Health Check
        </h2>
        <button className="button" disabled={busy} onClick={onCheck}>
          <RefreshCw size={14} />
          Run health check
        </button>
      </div>
      <div className="health-body">
        <span className={`badge ${health?.complete ? 'enabled' : 'needs_review'}`}>
          {health?.complete ? 'Scan evidence current' : 'Fresh scan recommended'}
        </span>
        <span>{health?.issues.length ?? 0} reported issues</span>
        <p>
          {health?.compatibility ||
            'A read-only check for missing dependencies, conflicts, incomplete scans and tracked-file problems. It does not certify in-game compatibility.'}
        </p>
        {!!health?.issues.length && (
          <details>
            <summary>
              <TriangleAlert size={15} />
              Inspect health findings
            </summary>
            {health.issues.map((issue, i) => (
              <p className="inline-warning" key={i}>
                <strong>{issue.subject}</strong> · {issue.detail}
              </p>
            ))}
          </details>
        )}
      </div>
    </section>
  );
}
