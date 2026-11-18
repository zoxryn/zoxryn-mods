"""Package verified artifacts; never bundles build caches or synthetic game files."""
from pathlib import Path
import shutil, zipfile, hashlib, json, subprocess
project=Path(__file__).resolve().parents[1]
output=project.parent
version=json.loads((project/'package.json').read_text())['version']
release=project/'src-tauri/target/release'
installer=release/f'bundle/nsis/Zoxryn Mods_{version}_x64-setup.exe'
shutil.copyfile(installer,output/f'Zoxryn-Mods-{version}-Setup.exe')
shutil.copyfile(Path(str(installer)+'.sig'),output/f'Zoxryn-Mods-{version}-Setup.exe.sig')
portable=output/f'Zoxryn-Mods-{version}'
portable.mkdir(exist_ok=True)
for name in ['zoxryn-mods.exe','WebView2Loader.dll']:
    shutil.copyfile(release/name,portable/name)
for name in ['README.md','THIRD_PARTY_NOTICES.md','UPDATES.md','ORGANIZATION.md']:
    shutil.copyfile(project/name,portable/name)
shutil.copytree(project/'licenses',portable/'licenses',dirs_exist_ok=True)
with zipfile.ZipFile(output/f'Zoxryn-Mods-{version}-Portable.zip','w',zipfile.ZIP_DEFLATED,compresslevel=9) as z:
    for p in portable.rglob('*'):
        if p.is_file(): z.write(p,Path(portable.name)/p.relative_to(portable))
ui_report=json.loads((project/'verification/interface-tests.json').read_text())
assert ui_report['stats']['unexpected']==0 and ui_report['stats']['expected']>=16, 'A passing full UI report is required'
subprocess.run(['node',str(project/'scripts/audit-public-source.mjs')],cwd=project,check=True)
source_files=sorted(set(subprocess.check_output(['git','ls-files','--cached','--others','--exclude-standard','-z'],cwd=project).decode().split('\0'))- {''})
with zipfile.ZipFile(output/f'Zoxryn-Mods-{version}-Source.zip','w',zipfile.ZIP_DEFLATED,compresslevel=9) as z:
    for name in source_files:
        p=project/name
        if p.suffix.lower() in {'.key','.pem','.pfx','.dpapi'} or p.name.startswith('.env'): raise RuntimeError(f'Refusing to package potential signing secrets: {p.name}')
        z.write(p,Path('zoxryn-mods')/p.relative_to(project))
artifacts={p.relative_to(output).as_posix():{'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in [output/f'Zoxryn-Mods-{version}-Setup.exe',output/f'Zoxryn-Mods-{version}-Setup.exe.sig',output/f'Zoxryn-Mods-{version}-Portable.zip',output/f'Zoxryn-Mods-{version}-Source.zip',portable/'zoxryn-mods.exe',portable/'WebView2Loader.dll']}
(output/f'SHA256SUMS-{version}.txt').write_text('\n'.join(f"{info['sha256']}  {name}" for name,info in artifacts.items())+'\n')
(output/f'release-manifest-{version}.json').write_text(json.dumps(artifacts,indent=2))
print(json.dumps(artifacts,indent=2))
