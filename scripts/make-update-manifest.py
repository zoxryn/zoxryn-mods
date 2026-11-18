"""Prepare a Tauri v2 feed. Sign the resulting bytes separately with with-signing.ps1."""
import argparse,json,hashlib
from pathlib import Path
from datetime import datetime,timezone,timedelta
p=argparse.ArgumentParser();p.add_argument('--artifact',required=True);p.add_argument('--version',required=True);p.add_argument('--url',required=True);p.add_argument('--output',required=True);p.add_argument('--notes',default='Zoxryn Mods application update.');p.add_argument('--valid-days',type=int,default=30);a=p.parse_args()
if not 1 <= a.valid_days <= 90: p.error('Metadata validity must be between 1 and 90 days')
artifact=Path(a.artifact);signature=Path(str(artifact)+'.sig').read_text().strip()
now=datetime.now(timezone.utc)
manifest={'version':a.version,'notes':a.notes,'pub_date':now.isoformat().replace('+00:00','Z'),'expires_at':(now+timedelta(days=a.valid_days)).isoformat().replace('+00:00','Z'),'platforms':{'windows-x86_64-nsis':{'signature':signature,'url':a.url,'size':artifact.stat().st_size,'sha256':hashlib.sha256(artifact.read_bytes()).hexdigest()}}}
out=Path(a.output);out.parent.mkdir(parents=True,exist_ok=True);out.write_text(json.dumps(manifest,indent=2)+'\n',encoding='utf-8');print(f'Prepared public manifest: {out.name}')
