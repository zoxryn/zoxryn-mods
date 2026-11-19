"""Deploy a reviewed copy to a dedicated Windows signer, never to a build runner.

Default policy is disabled. This process does not build or execute candidates.
GitHub App installation token must be injected by the host's protected token broker.
"""
from __future__ import annotations
import argparse, hashlib, json, os, platform, re, subprocess, sys, time, uuid
from datetime import datetime, timezone, timedelta
from pathlib import Path
from urllib.request import Request, urlopen
from urllib.error import HTTPError
from contextlib import contextmanager

REPO='zoxryn/zoxryn-mods'
WORKFLOW='.github/workflows/release-candidate.yml'
MAX_ARTIFACT=256*1024*1024
class Refused(RuntimeError): pass
@contextmanager
def exclusive_lock(path):
    handle=path.open('x')
    try:yield
    finally:
        handle.close()
        path.unlink() # Only the empty mutex created by this invocation.
def require(condition,reason):
    if not condition: raise Refused(reason)
def digest(path):
    with path.open('rb') as file:return hashlib.file_digest(file,'sha256').hexdigest()
def inputs(version,commit):
    require(re.fullmatch(r'\d+\.\d+\.\d+',version) is not None,'Stable version required')
    require(re.fullmatch(r'[0-9a-f]{40}',commit) is not None,'Full immutable commit required')
def six_names(version):
    name=f'Zoxryn-Mods-{version}-Setup.exe'
    return [name,name+'.sig','latest.json','latest.json.sig','SHA256SUMS.txt','RELEASE_NOTES.md']
def bounded(path,root,limit):
    current=path.absolute()
    while current!=current.parent:
        require(not current.is_symlink() and not getattr(current,'is_junction',lambda:False)(),'Release path cannot be a link')
        current=current.parent
    root=root.resolve(strict=True);path=path.resolve(strict=True)
    require(path.is_relative_to(root) and path.is_file() and path.stat().st_size<=limit,'Release file escaped its size/path budget')
    return path
def validate_candidate(folder,version,commit,request_id,public_key):
    inputs(version,commit)
    name=six_names(version)[0]
    require(sorted(p.name for p in folder.iterdir())==sorted([name,'candidate.json']),'Unexpected candidate files')
    meta=json.loads(bounded(folder/'candidate.json',folder,128*1024).read_text(encoding='utf-8'))
    artifact=bounded(folder/name,folder,MAX_ARTIFACT)
    require(meta['repository']==REPO and meta['version']==version and meta['commit']==commit and meta['request_id']==request_id,'Candidate identity mismatch')
    require(meta['public_key']==public_key and meta['artifact']==name,'Updater identity or artifact mismatch')
    require(meta['sha256']==digest(artifact) and meta['bytes']==artifact.stat().st_size,'Candidate checksum mismatch')
    with artifact.open('rb') as file: magic=file.read(2)
    require(magic==b'MZ' and artifact.stat().st_size>1024,'Invalid Windows installer')
    require(meta['tests']==dict.fromkeys(['backend','frontend','native_updater','release_policy','source_audit','dependency_audit'],True),'Required checks did not pass')
    require(isinstance(meta['notes'],str) and 0<len(meta['notes'])<32*1024,'Release notes missing or oversized')
    return meta
def make_manifest(folder,meta,now=None):
    now=now or datetime.now(timezone.utc)
    name=meta['artifact'];signature=(folder/(name+'.sig')).read_text().strip()
    require(len(signature)<4096,'Oversized updater signature')
    iso=lambda value:value.isoformat().replace('+00:00','Z')
    manifest={'version':meta['version'],'notes':meta['notes'],'pub_date':iso(now),'expires_at':iso(now+timedelta(days=30)),'platforms':{'windows-x86_64-nsis':{'url':f'https://github.com/{REPO}/releases/download/v{meta["version"]}/{name}','signature':signature,'size':meta['bytes'],'sha256':meta['sha256']}}}
    (folder/'latest.json').write_text(json.dumps(manifest,indent=2)+'\n',encoding='utf-8')
    (folder/'RELEASE_NOTES.md').write_text(meta['notes'],encoding='utf-8')
def finalize(folder,version):
    names=six_names(version)
    require(set(p.name for p in folder.iterdir())==set(names)-{'SHA256SUMS.txt'},'Only five reviewed production inputs may be checksummed')
    lines=[f'{digest(bounded(folder/name,folder,MAX_ARTIFACT))}  {name}' for name in names if name!='SHA256SUMS.txt']
    (folder/'SHA256SUMS.txt').write_text('\n'.join(lines)+'\n',encoding='utf-8')
    return {name:digest(folder/name) for name in names}
def check_expiry(folder):
    value=json.loads((folder/'latest.json').read_text())
    expiry=datetime.fromisoformat(value['expires_at'].replace('Z','+00:00'))
    require(datetime.now(timezone.utc)+timedelta(minutes=15)<expiry<=datetime.now(timezone.utc)+timedelta(days=31),'Manifest expired or excessive validity')
def release_state(release,version,commit,draft):
    require(release['tag_name']=='v'+version and release['target_commitish']==commit and release['draft']==draft and not release['prerelease'],'Unexpected release target or stable/draft state')
    require(sorted(a['name'] for a in release['assets'])==sorted(six_names(version)),'Public/draft asset set differs')

class GitHub:
    def __init__(self,policy): self.policy=policy;self.gh=policy['tools']['gh']['path']
    def command(self,*args,json_output=False):
        result=subprocess.run([self.gh,*args],capture_output=True,encoding='utf-8',cwd=self.policy['control_root'],creationflags=0x08000000)
        require(result.returncode==0,'GitHub command failed; inspect protected host diagnostics without exposing credentials')
        return json.loads(result.stdout) if json_output else result.stdout
    def api(self,path,optional=False):
        url='https://api.github.com/repos/'+REPO+'/'+path
        try:
            with urlopen(Request(url,headers={'Authorization':'Bearer '+os.environ['GH_TOKEN'],'Accept':'application/vnd.github+json','X-GitHub-Api-Version':'2022-11-28'}),timeout=30) as response:
                raw=response.read(4*1024*1024+1);require(len(raw)<=4*1024*1024,'GitHub response exceeded budget');return json.loads(raw)
        except HTTPError as e:
            if optional and e.code==404:return None
            raise Refused('GitHub API failed; production publication stopped') from None
    def preflight(self,version,commit):
        require(self.api('')['id']==self.policy['repository_id'],'Repository numeric identity changed')
        require(self.api('git/ref/heads/main')['object']['sha']==commit,'Main no longer matches approved commit')
        require(self.api('releases/tags/v'+version,True) is None and self.api('git/ref/tags/v'+version,True) is None,'Version or tag already exists; never overwrite')
        latest=self.api('releases/latest',True)
        if latest:
            previous=latest['tag_name'].removeprefix('v');inputs(previous,commit)
            require(tuple(map(int,version.split('.')))>tuple(map(int,previous.split('.'))),'Version is not newer than stable release')
    def build(self,version,commit,request_id,folder):
        self.command('workflow','run',WORKFLOW,'--repo',REPO,'--ref','main','-f','version='+version,'-f','commit='+commit,'-f','request_id='+request_id)
        deadline=time.monotonic()+65*60;run=None
        while time.monotonic()<deadline:
            runs=self.api('actions/workflows/release-candidate.yml/runs?event=workflow_dispatch&branch=main&per_page=30')['workflow_runs']
            matches=[r for r in runs if r['display_title']==f'candidate {version} {request_id}']
            require(len(matches)<=1,'Ambiguous release workflow runs')
            if matches:
                run=matches[0]
                require(run['head_sha']==commit and run['event']=='workflow_dispatch' and run['head_branch']=='main' and run.get('run_attempt',1)==1,'Unexpected CI source or rerun')
                if run['status']=='completed':break
            time.sleep(15)
        require(run is not None and run['status']=='completed' and run['conclusion']=='success','CI failed, timed out or was cancelled')
        artifacts=self.api(f'actions/runs/{run["id"]}/artifacts')['artifacts']
        require(len(artifacts)==1 and artifacts[0]['name']=='candidate-'+request_id and not artifacts[0]['expired'],'Unexpected or expired candidate artifact')
        self.command('run','download',str(run['id']),'--repo',REPO,'--name','candidate-'+request_id,'--dir',str(folder))
        return run['id']
    def attest(self,path,commit):
        self.command('attestation','verify',str(path),'--repo',REPO,'--cert-identity',f'https://github.com/{REPO}/{WORKFLOW}@refs/heads/main','--source-digest',commit,'--signer-digest',commit,'--source-ref','refs/heads/main','--deny-self-hosted-runners')
    def publish(self,folder,version,commit,hashes):
        self.preflight(version,commit);check_expiry(folder)
        require(all(digest(folder/name)==value for name,value in hashes.items()),'Files changed after signature verification')
        tag='v'+version
        self.command('release','create',tag,'--repo',REPO,'--target',commit,'--draft','--title','Zoxryn Mods '+tag,'--notes-file',str(folder/'RELEASE_NOTES.md'))
        self.command('release','upload',tag,*[str(folder/n) for n in six_names(version)],'--repo',REPO)
        release_state(self.api('releases/tags/'+tag),version,commit,True)
        require(self.api('git/ref/heads/main')['object']['sha']==commit,'Main changed before final publication')
        check_expiry(folder)
        self.command('release','edit',tag,'--repo',REPO,'--draft=false','--prerelease=false','--latest')
        release=self.api('releases/tags/'+tag);release_state(release,version,commit,False)
        require(self.api('git/ref/tags/'+tag)['object']['sha']==commit,'Published tag differs from approved commit')
        return release
    def public_verify(self,folder,version,hashes):
        public=folder.parent/'public-verification';public.mkdir()
        urls={name:f'https://github.com/{REPO}/releases/download/v{version}/{name}' for name in hashes}
        for name,url in urls.items():
            with urlopen(url,timeout=60) as response:
                data=response.read(MAX_ARTIFACT+1);require(len(data)<=MAX_ARTIFACT,'Public download exceeded limit')
            require(hashlib.sha256(data).hexdigest()==hashes[name],'Public download checksum mismatch')
            (public/name).write_bytes(data)
        with urlopen(f'https://github.com/{REPO}/releases/latest/download/latest.json',timeout=60) as response:feed=response.read(64*1024+1)
        require(feed==(public/'latest.json').read_bytes(),'Public latest feed is not the published signed manifest')
        check_expiry(public)
        return public

def trusted_tools(policy):
    require(policy.get('enabled') is True,'Production automation is disabled; explicit security activation required')
    require(os.name=='nt' and platform.node().lower()==policy['host'].lower(),'Dedicated signing host required')
    require(policy['repository']==REPO and policy['repository_id']>0,'Repository policy is unconfigured')
    require(Path(__file__).resolve().parent==Path(policy['control_root']).resolve(),'Run the protected deployed coordinator, not a repository checkout')
    require(sys.flags.isolated==1 and Path(sys.executable).resolve()==Path(policy['tools']['python']['path']).resolve(),'Use the pinned Python runtime in isolated mode')
    for tool in policy['tools'].values():
        path=bounded(Path(tool['path']),Path(policy['control_root']),MAX_ARTIFACT)
        require(re.fullmatch('[0-9a-f]{64}',tool['sha256']) is not None and digest(path)==tool['sha256'],'Pinned release tool changed')
    require(os.environ.get('GH_TOKEN'),'Protected short-lived GitHub App token required')
def verify(policy,folder,version):
    result=subprocess.run([policy['tools']['verifier']['path'],str(folder/six_names(version)[0]),version,str(folder/'latest.json')],cwd=policy['control_root'],capture_output=True,creationflags=0x08000000)
    require(result.returncode==0,'Native updater signature, version, expiry or SHA-256 verification failed')
def sign(policy,policy_path,path,version):
    environment={k:v for k,v in os.environ.items() if not k.upper().startswith(('GH_','GITHUB_','TAURI_SIGNING_'))}
    powershell=Path(os.environ['SystemRoot'])/'System32/WindowsPowerShell/v1.0/powershell.exe'
    result=subprocess.run([str(powershell),'-NoProfile','-NonInteractive','-File',policy['tools']['sign_bridge']['path'],'-Policy',str(policy_path),'-Artifact',str(path),'-Version',version],cwd=policy['control_root'],env=environment,capture_output=True,creationflags=0x08000000)
    require(result.returncode==0,'Isolated signing failed; no signature output or secret was logged')
def main():
    parser=argparse.ArgumentParser();parser.add_argument('--policy',required=True,type=Path);parser.add_argument('--version',required=True);parser.add_argument('--commit',required=True);parser.add_argument('--approve-release',required=True)
    args=parser.parse_args();inputs(args.version,args.commit)
    require(args.approve_release==f'v{args.version}@{args.commit}','Exact authorized release trigger required')
    policy=json.loads(args.policy.read_text());trusted_tools(policy)
    bounded(args.policy,Path(policy['control_root']),64*1024)
    github=GitHub(policy);github.preflight(args.version,args.commit)
    request_id=uuid.uuid4().hex;root=Path(policy['work_root'])/request_id;root.mkdir(parents=True,exist_ok=False)
    lock=Path(policy['work_root'])/'release.lock'
    with exclusive_lock(lock):
        try:
            candidate=root/'candidate';candidate.mkdir();run_id=github.build(args.version,args.commit,request_id,candidate)
            for path in candidate.iterdir():github.attest(path,args.commit)
            meta=validate_candidate(candidate,args.version,args.commit,request_id,policy['public_key'])
            production=root/'production';production.mkdir();(production/meta['artifact']).write_bytes((candidate/meta['artifact']).read_bytes())
            sign(policy,args.policy,production/meta['artifact'],args.version);make_manifest(production,meta);sign(policy,args.policy,production/'latest.json',args.version)
            verify(policy,production,args.version);hashes=finalize(production,args.version)
            release=github.publish(production,args.version,args.commit,hashes)
            public=github.public_verify(production,args.version,hashes);verify(policy,public,args.version)
            (root/'receipt.json').write_text(json.dumps({'version':args.version,'commit':args.commit,'run_id':run_id,'release':release['html_url'],'assets':hashes,'public_verified':True},indent=2)+'\n')
            print('Stable release and public update feed verified: '+release['html_url'])
        except Exception:
            (root/'FAILED').write_text('Release stopped. Retain this directory and inspect GitHub draft/public state before retry. No overwrite or deletion is automatic.\n')
            raise
if __name__=='__main__':
    try:main()
    except Exception as error:raise SystemExit('Release coordinator stopped: '+(str(error) if isinstance(error,Refused) else type(error).__name__))
