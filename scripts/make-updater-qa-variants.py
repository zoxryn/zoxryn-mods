"""Author negative metadata cases only inside an explicitly marked synthetic QA root."""
import argparse, json
from pathlib import Path
from datetime import datetime, timezone, timedelta
p=argparse.ArgumentParser(); p.add_argument('root',type=Path); a=p.parse_args(); root=a.root.resolve()
assert (root/'SYNTHETIC-UPDATER-QA').read_text().strip()=='ZOXRYN-SYNTHETIC-ONLY'
raw=json.loads((root/'latest.json').read_text(encoding='utf-8-sig'))
now=datetime.now(timezone.utc)
cases={}
expired=json.loads(json.dumps(raw)); expired['pub_date']=(now-timedelta(days=2)).isoformat(); expired['expires_at']=(now-timedelta(days=1)).isoformat(); cases['expired']=expired
downgrade=json.loads(json.dumps(raw)); downgrade['version']='0.1.6'; cases['downgrade']=downgrade
prerelease=json.loads(json.dumps(raw)); prerelease['version']=raw['version']+'-beta.1'; cases['prerelease']=prerelease
invalid=json.loads(json.dumps(raw)); invalid['notes']={'malformed':'must be text'}; cases['invalid']=invalid
wrong=json.loads(json.dumps(raw)); wrong['platforms']['windows-x86_64-nsis']['signature']=(root/'source.exe.sig').read_text().strip(); cases['wrong-artifact-version']=wrong
for name, manifest in cases.items(): (root/(name+'.json')).write_text(json.dumps(manifest,indent=2)+'\n',encoding='utf8')
updates=root/'data/app-updates'; updates.mkdir(exist_ok=True)
(updates/'receipt.json').write_text(json.dumps({'from_version':'0.1.6','to_version':'0.0.0','phase':'installing','database_backup':'','message':None}))
print('Prepared negative metadata inputs and a simulated incomplete receipt in synthetic data only.')
