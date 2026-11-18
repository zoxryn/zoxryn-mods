"""Collect dependency licenses and identify the active Windows Cargo graph."""
import json, shutil, sys, re
from pathlib import Path
root=Path(__file__).resolve().parents[1]
metadata=json.loads(Path(sys.argv[1]).read_text(encoding='utf-8-sig'))
nodes={n['id']: n for n in metadata['resolve']['nodes']}
active=set()
def visit(pid):
    if pid in active: return
    active.add(pid)
    for dep in nodes.get(pid,{}).get('deps',[]): visit(dep['pkg'])
visit(metadata['resolve']['root'])
if len(sys.argv)>2:
    exact=set(re.findall(r'^([\w-]+) v([\w.+-]+)',Path(sys.argv[2]).read_text(encoding='utf-8-sig'),re.M))
    packages=[p for p in metadata['packages'] if (p['name'],p['version']) in exact and p.get('source')]
else:
    packages=[p for p in metadata['packages'] if p['id'] in active and p.get('source')]
licenses=root/'licenses'
licenses.mkdir(exist_ok=True)
lines=['# Third-party notices','', 'Zoxryn Mods includes software under the following licenses. Corresponding license files are supplied in `licenses/`. Original Zoxryn application code and icon were created for this project.', '', '## Rust dependencies in the Windows build', '', '| Package | Version | License |','|---|---|---|']
for p in sorted(packages,key=lambda p:p['name']):
    lines.append(f"| {p['name']} | {p['version']} | {p.get('license') or p.get('license_file') or 'See package'} |")
    src=Path(p['manifest_path']).parent
    for f in src.iterdir():
        if f.is_file() and (f.name.upper().startswith(('LICENSE','COPYING','COPYRIGHT','NOTICE'))):
            dest=licenses/f"rust-{p['name']}-{p['version']}"
            dest.mkdir(exist_ok=True)
            shutil.copyfile(f,dest/f.name)
lines.extend(['','## JavaScript runtime dependencies','','| Package | Version | License |','|---|---|---|'])
for name in ['react','react-dom','scheduler','lucide-react','@tauri-apps/api','@tauri-apps/plugin-dialog']:
    src=root/'node_modules'/name
    if not src.exists():
        matches=list((root/'node_modules/.pnpm').glob(f'{name}@*/node_modules/{name}'))
        if not matches: continue
        src=matches[0]
    p=json.loads((src/'package.json').read_text())
    lines.append(f"| {name} | {p['version']} | {p.get('license','See package')} |")
    for f in src.iterdir():
        if f.is_file() and f.name.upper().startswith(('LICENSE','COPYING','COPYRIGHT','NOTICE')):
            dest=licenses/f"js-{name.replace('/','-')}"
            dest.mkdir(exist_ok=True)
            shutil.copyfile(f,dest/f.name)
toolchain=root.parents[1]/'work/toolchain'
for f in (toolchain/'w64devkit').glob('COPYING*'):
    if 'runtime' in f.name.lower(): shutil.copyfile(f,licenses/f.name)
lines.extend(['','The GNU build also uses the Rust standard library (MIT OR Apache-2.0) and MinGW-w64 runtime. The original Rust and MinGW runtime license and exception texts are included when supplied by the toolchain. WebView2 is a separately installed Microsoft runtime.'])
rust=toolchain/'rustup/toolchains/stable-x86_64-pc-windows-gnu/share/doc/rust'
for pattern in ['COPYRIGHT','LICENSE*']:
    for f in rust.glob(pattern):
        if f.is_file(): shutil.copyfile(f,licenses/f'rust-std-{f.name}')
(root/'THIRD_PARTY_NOTICES.md').write_text('\n'.join(lines)+'\n',encoding='utf-8')
report=root/'verification/rust-advisories.json'
if report.exists():
    data=json.loads(report.read_text())
    versions={f"{p['name']} {p['version']}" for p in packages}
    for f in data['findings']: f['active_in_windows_build']=f['package'] in versions
    data['active_windows_packages']=len(packages)
    report.write_text(json.dumps(data,indent=2))
print(f'Collected notices for {len(packages)} active Windows Rust packages. Advisory matches in active graph: '+str(sum(f['active_in_windows_build'] for f in data['findings'])))
