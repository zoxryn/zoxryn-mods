import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, copyFile, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
test('the source audit inspects staged bytes even after the working copy is cleaned', async () => {
  const root = await mkdtemp(join(tmpdir(), 'zoxryn-source-audit-'));
  try {
    await mkdir(join(root, 'scripts'));
    await copyFile(resolve('scripts/audit-public-source.mjs'), join(root, 'scripts/audit-public-source.mjs'));
    const git = args => { const r=spawnSync('git',args,{cwd:root,windowsHide:true}); assert.equal(r.status,0); };
    git(['init','--initial-branch=main']);
    await writeFile(join(root,'notes.txt'), ['C:', 'Users', 'SYNTHETIC_PERSON', 'private-folder'].join('\\'));
    git(['add','notes.txt']);
    await writeFile(join(root,'notes.txt'),'Safe public content.');
    const staged=spawnSync(process.execPath,['scripts/audit-public-source.mjs','--staged'],{cwd:root,encoding:'utf8',windowsHide:true});
    assert.equal(staged.status,1,staged.stderr);
    assert.ok(JSON.parse(staged.stdout).findings.some(f=>f.kind==='private-directory'));
    assert.ok(!staged.stdout.includes('SYNTHETIC_PERSON'));
    git(['add','notes.txt']);
    const clean=spawnSync(process.execPath,['scripts/audit-public-source.mjs','--staged'],{cwd:root,encoding:'utf8',windowsHide:true});
    assert.equal(clean.status,0,clean.stderr);
    assert.equal(JSON.parse(clean.stdout).passed,true);
  } finally {
    // This is the exact directory returned by mkdtemp, never a computed user path.
    assert.ok(root.startsWith(join(tmpdir(),'zoxryn-source-audit-')));
    await rm(root,{recursive:true,force:true});
  }
});
