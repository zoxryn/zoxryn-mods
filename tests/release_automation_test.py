"""Offline release fixtures only: never call GitHub or the production signer."""
import importlib.util,json,sys,tempfile,unittest
from pathlib import Path
from datetime import datetime,timezone,timedelta
from unittest.mock import patch
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('coordinator',ROOT/'scripts/release/coordinator.py');c=importlib.util.module_from_spec(spec);spec.loader.exec_module(c)
class ReleasePolicy(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup);self.folder=Path(self.temp.name)/'production';self.folder.mkdir()
        self.version='9.8.7';self.commit='a'*40;self.request='b'*32;self.public='SYNTHETIC PUBLIC IDENTITY ONLY'
        artifact=self.folder/c.six_names(self.version)[0];artifact.write_bytes(b'MZ'+b'SYNTHETIC NON-EXECUTABLE FIXTURE'*100)
        self.meta={'repository':c.REPO,'version':self.version,'commit':self.commit,'request_id':self.request,'artifact':artifact.name,'sha256':c.digest(artifact),'bytes':artifact.stat().st_size,'public_key':self.public,'notes':'Synthetic release notes.','tests':dict.fromkeys(['backend','frontend','native_updater','release_policy','source_audit','dependency_audit'],True)}
        self.write_meta()
    def write_meta(self):(self.folder/'candidate.json').write_text(json.dumps(self.meta))
    def validate(self):return c.validate_candidate(self.folder,self.version,self.commit,self.request,self.public)
    def production(self):
        self.validate();(self.folder/'candidate.json').unlink();(self.folder/(self.meta['artifact']+'.sig')).write_text('SYNTHETIC SIGNATURE ONLY');c.make_manifest(self.folder,self.meta);(self.folder/'latest.json.sig').write_text('SYNTHETIC SIGNATURE ONLY');return c.finalize(self.folder,self.version)
    def test_exact_identity_and_checksum(self):self.assertEqual(self.validate(),self.meta)
    def test_tampered_installer(self):
        (self.folder/self.meta['artifact']).write_bytes(b'MZ'+b'altered'*500)
        with self.assertRaises(c.Refused):self.validate()
    def test_identity_changes_rejected(self):
        for key,value in [('commit','c'*40),('public_key','other'),('version','9.8.8'),('request_id','d'*32),('repository','other/project')]:
            with self.subTest(key=key):
                old=self.meta[key];self.meta[key]=value;self.write_meta()
                with self.assertRaises(c.Refused):self.validate()
                self.meta[key]=old
    def test_failed_required_checks(self):
        self.meta['tests']['native_updater']=False;self.write_meta()
        with self.assertRaises(c.Refused):self.validate()
    def test_private_and_extra_assets_rejected(self):
        (self.folder/'private.fixture').write_text('Synthetic secret fixture')
        with self.assertRaises(c.Refused):self.validate()
    def test_traversal_and_size_budget(self):
        with self.assertRaises(c.Refused):c.bounded(ROOT/'package.json',self.folder,999999)
        with self.assertRaises(c.Refused):c.bounded(self.folder/self.meta['artifact'],self.folder,1)
    def test_manifest_and_six_assets(self):
        hashes=self.production();self.assertEqual(set(hashes),set(c.six_names(self.version)));c.check_expiry(self.folder)
        manifest=json.loads((self.folder/'latest.json').read_text());self.assertEqual(manifest['version'],self.version);self.assertEqual(manifest['platforms']['windows-x86_64-nsis']['sha256'],self.meta['sha256'])
        for line in (self.folder/'SHA256SUMS.txt').read_text().splitlines():
            value,name=line.split('  ');self.assertEqual(value,c.digest(self.folder/name))
    def test_expired_manifest(self):
        self.production();manifest=json.loads((self.folder/'latest.json').read_text());manifest['expires_at']=(datetime.now(timezone.utc)-timedelta(seconds=1)).isoformat();(self.folder/'latest.json').write_text(json.dumps(manifest))
        with self.assertRaises(c.Refused):c.check_expiry(self.folder)
    def test_draft_commit_prerelease_and_asset_collisions(self):
        release={'tag_name':'v'+self.version,'target_commitish':self.commit,'draft':True,'prerelease':False,'assets':[{'name':n} for n in c.six_names(self.version)]};c.release_state(release,self.version,self.commit,True)
        for key,value in [('target_commitish','c'*40),('prerelease',True),('draft',False),('assets',[])]:
            modified={**release,key:value}
            with self.assertRaises(c.Refused):c.release_state(modified,self.version,self.commit,True)
    def test_production_disabled_without_host_configuration(self):
        policy=json.loads((ROOT/'scripts/release/production-policy.example.json').read_text())
        with self.assertRaisesRegex(c.Refused,'disabled'):c.trusted_tools(policy)
    def test_full_sha_and_stable_versions_only(self):
        for version,commit in [('1.2.3-beta',self.commit),('1.2.3','main'),('1.2.3;command',self.commit)]:
            with self.assertRaises(c.Refused):c.inputs(version,commit)
    def test_attestation_policy_requires_exact_san_source_and_hosted_builder(self):
        github=c.GitHub({'tools':{'gh':{'path':'unused'}},'control_root':str(self.folder)})
        with patch.object(github,'command') as run:
            github.attest(self.folder/'fixture',self.commit)
            args=run.call_args.args
            self.assertIn('--cert-identity',args);self.assertIn('--source-digest',args);self.assertIn('--signer-digest',args);self.assertIn('--deny-self-hosted-runners',args)
            self.assertIn('https://github.com/'+c.REPO+'/'+c.WORKFLOW+'@refs/heads/main',args)
    def test_preflight_rejects_changed_main_and_existing_tag(self):
        github=c.GitHub({'repository_id':123,'tools':{'gh':{'path':'unused'}},'control_root':str(self.folder)})
        with patch.object(github,'api',side_effect=[{'id':123},{'object':{'sha':'f'*40}}]):
            with self.assertRaisesRegex(c.Refused,'Main'):github.preflight(self.version,self.commit)
        with patch.object(github,'api',side_effect=[{'id':123},{'object':{'sha':self.commit}},{'id':1}]):
            with self.assertRaisesRegex(c.Refused,'already exists'):github.preflight(self.version,self.commit)
    def test_workflow_has_no_ordinary_or_untrusted_release_trigger(self):
        source=(ROOT/'.github/workflows/release-candidate.yml').read_text()
        self.assertIn('workflow_dispatch:',source)
        for forbidden in ['pull_request:','pull_request_target:','push:','contents: write','secrets.','self-hosted']:
            self.assertNotIn(forbidden,source)
        import re
        actions=re.findall(r'uses: (\S+)',source);self.assertEqual(len(actions),5)
        self.assertTrue(all(re.fullmatch(r'actions/[a-z-]+@[0-9a-f]{40}',action) for action in actions))
    def test_complete_synthetic_publication_sequence_and_public_downloads(self):
        hashes=self.production();release={'tag_name':'v'+self.version,'target_commitish':self.commit,'draft':True,'prerelease':False,'assets':[{'name':n} for n in c.six_names(self.version)],'html_url':'https://github.com/'+c.REPO+'/releases/tag/v'+self.version}
        github=c.GitHub({'tools':{'gh':{'path':'unused'}},'control_root':str(self.folder)})
        public={**release,'draft':False}
        with patch.object(github,'preflight'),patch.object(github,'command') as commands,patch.object(github,'api',side_effect=[release,{'object':{'sha':self.commit}},public,{'object':{'sha':self.commit}}]):
            self.assertEqual(github.publish(self.folder,self.version,self.commit,hashes),public)
            called=[call.args for call in commands.call_args_list];self.assertEqual([args[1] for args in called],['create','upload','edit']);self.assertIn('--draft',called[0]);self.assertIn('--prerelease=false',called[2]);self.assertNotIn('--clobber',str(called))
        class Response:
            def __init__(self,data):self.data=data
            def __enter__(self):return self
            def __exit__(self,*args):return None
            def read(self,limit):return self.data[:limit]
        with patch.object(c,'urlopen',side_effect=lambda url,**kwargs:Response((self.folder/url.rsplit('/',1)[-1]).read_bytes())):
            folder=github.public_verify(self.folder,self.version,hashes)
            self.assertEqual({name:c.digest(folder/name) for name in hashes},hashes)
    def test_failed_upload_never_reaches_publish_step(self):
        hashes=self.production();github=c.GitHub({'tools':{'gh':{'path':'unused'}},'control_root':str(self.folder)})
        with patch.object(github,'preflight'),patch.object(github,'command',side_effect=[None,c.Refused('Synthetic network failure')]) as commands:
            with self.assertRaisesRegex(c.Refused,'network'):github.publish(self.folder,self.version,self.commit,hashes)
            self.assertEqual(commands.call_count,2)
    def test_exclusive_mutex_blocks_collisions_and_closes_before_removing(self):
        path=self.folder/'release.lock'
        with c.exclusive_lock(path):
            with self.assertRaises(FileExistsError):
                with c.exclusive_lock(path):self.fail('Duplicate publisher entered')
        self.assertFalse(path.exists())
        with c.exclusive_lock(path):self.assertTrue(path.exists())
if __name__=='__main__':unittest.main()
