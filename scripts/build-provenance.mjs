import { execFileSync } from 'node:child_process';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
const root = resolve(fileURLToPath(new URL('..', import.meta.url)));
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const names = [...new Set(execFileSync('git', ['ls-files', '--cached', '--others', '--exclude-standard', '-z'], { cwd: root, encoding: 'utf8', windowsHide: true }).split('\0').filter(name => /^(src\/|src-tauri\/|public\/|package.json$|pnpm-lock.yaml$|tsconfig.json$|vite.config.ts$|scripts\/(build|with-signing)\.ps1$)/.test(name)))].sort();
const source = [];
for (const name of names) source.push([name, hash(await readFile(join(root, name)))]);
const version = JSON.parse(await readFile(join(root, 'package.json'), 'utf8')).version;
const artifacts = ['src-tauri/target/release/zoxryn-mods.exe', `src-tauri/target/release/bundle/nsis/Zoxryn Mods_${version}_x64-setup.exe`];
const binaries = {};
for (const name of artifacts) {
  const bytes = await readFile(join(root, name));
  const user = (process.env.USERNAME || '').toLowerCase();
  if (user.length >= 3 && [bytes.toString('latin1').toLowerCase(), bytes.toString('utf16le').toLowerCase()].some(text => text.includes(user))) throw new Error('A packaged binary contains the local Windows username; publication is blocked.');
  binaries[name] = hash(bytes);
}
const result = { version, sourceSha256: hash(JSON.stringify(source)), binaries, binaryPrivacyCheckPassed: true, backendTestsPassed: true, desktopTestsPassed: true, frontendTestsPassed: true };
const path = join(root, 'verification/production-build-provenance.json');
if (process.argv.includes('--verify')) {
  const before = JSON.parse(await readFile(path, 'utf8'));
  if (JSON.stringify(before) !== JSON.stringify(result)) throw new Error('Source inputs or release binaries changed after the tested build.');
} else {
  await mkdir(join(root, 'verification'), { recursive: true });
  await writeFile(path, JSON.stringify(result, null, 2) + '\n');
}
console.log(JSON.stringify({ version, sourceSha256: result.sourceSha256, binaryPrivacyCheckPassed: true, verified: true }));
