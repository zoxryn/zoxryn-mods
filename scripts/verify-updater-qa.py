"""Capture/verify logical data and byte-for-byte files in the synthetic updater QA root."""
import argparse, hashlib, json, sqlite3
from pathlib import Path

p = argparse.ArgumentParser()
p.add_argument('action', choices=['capture', 'verify'])
p.add_argument('root', type=Path)
a = p.parse_args()
root = a.root.resolve()
assert (root / 'SYNTHETIC-UPDATER-QA').read_text().strip() == 'ZOXRYN-SYNTHETIC-ONLY'

def snapshot():
    files = {}
    for name in ['Mock GTA V Legacy', 'downloads', 'data/archives', 'data/blobs', 'data/journals', 'data/thumbnails']:
        for f in (root / name).rglob('*'):
            if f.is_file():
                assert not f.is_symlink()
                stat=f.stat()
                files[f.relative_to(root).as_posix()] = {'sha256':hashlib.sha256(f.read_bytes()).hexdigest(),'bytes':stat.st_size,'mtime_ns':stat.st_mtime_ns}
    with sqlite3.connect(root / 'data/zoxryn.sqlite3') as db:
        tables = {t: db.execute(f'SELECT * FROM {t} ORDER BY 1,2').fetchall()
                  for t in ['settings', 'mods', 'owned', 'journals', 'scan_runs', 'external_mods', 'collections', 'collection_members', 'visual_metadata']}
        schema = db.execute('PRAGMA user_version').fetchone()[0]
        assert db.execute('PRAGMA integrity_check').fetchone()[0] == 'ok'
    return json.loads(json.dumps({'files': files, 'tables': tables, 'schema': schema}))

if a.action == 'capture':
    baseline=snapshot()
    assert baseline['schema']==3
    assert len(baseline['tables']['collections'])==1 and len(baseline['tables']['collection_members'])==2
    assert len(baseline['tables']['visual_metadata'])==1
    assert any(name.startswith('data/thumbnails/') for name in baseline['files'])
    (root / 'baseline.json').write_text(json.dumps(baseline, indent=2))
    print('Captured synthetic SQLite records and file hashes.')
else:
    before = json.loads((root / 'baseline.json').read_text())
    assert snapshot() == before, 'Synthetic records/files changed during the application update'
    assert not (root / 'evidence/failed.json').exists(), (root / 'evidence/failed.json').read_text() if (root / 'evidence/failed.json').exists() else ''
    old = json.loads((root / 'evidence/before-install.json').read_text())
    new = json.loads((root / 'evidence/relaunch.json').read_text())
    receipt = json.loads((root / 'data/app-updates/receipt.json').read_text())
    versions=json.loads((root/'versions.json').read_text(encoding='utf-8-sig'))
    assert old['version'] == versions['from'] and new['version'] == versions['to']
    assert old['identifier'] == new['identifier'] == 'com.zoxryn.mods.updater-qa'
    assert new['launched'] and new['mods'] == 3 and new['backups'] == 3
    assert Path(new['data']).samefile(root / 'data')
    assert Path(new['executable']).parent.samefile(root / 'Installed QA App')
    assert receipt['phase'] == 'completed'
    assert new['last_checked_at'] > 0
    assert new['last_check_restored_before_check'] > 0 and new['up_to_date_verified']
    backup = Path(receipt['database_backup'])
    assert backup.is_file() and backup.parent.samefile(root / 'data/app-update-backups')
    with sqlite3.connect(backup) as db:
        for table, rows in before['tables'].items():
            assert json.loads(json.dumps(db.execute(f'SELECT * FROM {table} ORDER BY 1,2').fetchall())) == rows
    requests = json.loads((root / 'requests.json').read_text())
    downloads = [r for r in requests if r['path'] == '/installer.exe']
    assert [r['scenario'] for r in downloads] == ['tampered_installer','interrupted_download','wrong_artifact_version','valid']
    assert all(c['blocked'] for c in old['checks']) and len(old['checks']) == 13
    progress=json.loads((root/'evidence/progress.json').read_text())
    assert any(e['phase']=='downloading' and e['downloaded_bytes']>0 for e in progress)
    assert any(e['phase']=='verifying' and e['downloaded_bytes']==e['total_bytes'] for e in progress)
    report = {'passed': True, 'from': old['version'], 'to': new['version'],
              'identifier_preserved': True, 'install_directory_preserved': True,
              'relaunch_verified': True, 'receipt_completed': True,
              'sqlite_schema': before['schema'], 'logical_tables_preserved': list(before['tables']),
              'unchanged_files': len(before['files']), 'database_backup_verified': True,
              'native_progress_verified':True,'last_check_survived_restart':True,
              'retained_file_timestamps_preserved':True,
              'negative_tests': old['checks'], 'installer_downloads': downloads,
              'scope': 'Disposable QA product/executable, synthetic game/data, loopback server; no production application or game accessed.'}
    (root / 'verification.json').write_text(json.dumps(report, indent=2))
    print(json.dumps(report, indent=2))
