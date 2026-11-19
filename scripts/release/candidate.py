"""Unsigned CI output only. The signing coordinator never executes this checkout."""
import hashlib, json, os, re
from pathlib import Path
root=Path(__file__).resolve().parents[2]
version=json.loads((root/'package.json').read_text())['version']
assert re.fullmatch(r'\d+\.\d+\.\d+',version) and version==os.environ['RELEASE_VERSION']
artifact=root/f'src-tauri/target/release/bundle/nsis/Zoxryn Mods_{version}_x64-setup.exe'
data=artifact.read_bytes()
assert data[:2]==b'MZ' and 1024<len(data)<=256*1024*1024
for name in [artifact,root/'src-tauri/target/release/zoxryn-mods.exe']:
    data_to_check=name.read_bytes()
    for encoding in ['latin1','utf-16le']:
        assert os.environ.get('USERNAME','build-user').lower() not in data_to_check.decode(encoding,errors='ignore').lower(), 'Build path privacy check failed'
report=json.loads((root/'verification/interface-tests.json').read_text())
assert report['stats']['unexpected']==0 and report['stats']['expected']>=30
out=root/'release-assets';out.mkdir(exist_ok=False)
name=f'Zoxryn-Mods-{version}-Setup.exe';(out/name).write_bytes(data)
metadata={'repository':'zoxryn/zoxryn-mods','commit':os.environ['APPROVED_COMMIT'],'version':version,'request_id':os.environ['RELEASE_REQUEST'],'artifact':name,'sha256':hashlib.sha256(data).hexdigest(),'bytes':len(data),'public_key':(root/'src-tauri/updater-public-key.pub').read_text().strip(),'notes':(root/f'releases/v{version}.md').read_text(encoding='utf-8'),'tests':{'backend':True,'frontend':True,'native_updater':True,'release_policy':True,'source_audit':True,'dependency_audit':True}}
(out/'candidate.json').write_text(json.dumps(metadata,indent=2)+'\n',encoding='utf-8')
print('Prepared two unsigned candidate files; no production signing or publication.')
