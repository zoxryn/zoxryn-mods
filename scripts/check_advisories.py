"""Query OSV for exact locked Rust package versions; package names/versions only."""
import argparse, json, re, subprocess, tomllib, urllib.request
from pathlib import Path
root = Path(__file__).resolve().parents[1]
parser=argparse.ArgumentParser();parser.add_argument('--enforce-windows',action='store_true');args=parser.parse_args()
packages = [p for p in tomllib.loads((root/'src-tauri/Cargo.lock').read_text())['package'] if p.get('source','').startswith('registry+')]
queries = [{'package': {'name': p['name'], 'ecosystem': 'crates.io'}, 'version': p['version']} for p in packages]
request = urllib.request.Request('https://api.osv.dev/v1/querybatch', data=json.dumps({'queries': queries}).encode(), headers={'Content-Type': 'application/json'})
with urllib.request.urlopen(request, timeout=50) as response:
    data = json.load(response)
findings = [{'package': f"{p['name']} {p['version']}", 'advisories': r.get('vulns', [])} for p,r in zip(packages, data['results']) if r.get('vulns')]
output = {'service': 'https://api.osv.dev/v1/querybatch', 'packages_queried':len(packages), 'scope':'All registry packages in Cargo.lock, including build and non-Windows dependencies', 'findings':findings}
if args.enforce_windows:
    tree=subprocess.check_output(['cargo','tree','--locked','--features','desktop','--target','x86_64-pc-windows-msvc','--prefix','none','--format','{p}'],cwd=root/'src-tauri',text=True,creationflags=0x08000000)
    active={f'{name} {version}' for name,version in re.findall(r'^([A-Za-z0-9_-]+) v([^\s]+)',tree,re.M)}
    assert len(active)>100, 'Windows dependency coverage was not established'
    for finding in findings: finding['compiled_for_windows_desktop']=finding['package'] in active
    output['windows_packages_checked']=len(active)
    output['windows_gate_passed']=not any(f['compiled_for_windows_desktop'] for f in findings)
(root/'verification').mkdir(exist_ok=True)
(root/'verification/rust-advisories.json').write_text(json.dumps(output, indent=2))
print(json.dumps(output, indent=2))
if args.enforce_windows and not output['windows_gate_passed']:raise SystemExit('Advisories affect a Windows desktop dependency; release blocked for review')
