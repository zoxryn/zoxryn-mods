// Read-only audit. Findings never echo matched secret values or private paths.
import { execFileSync } from 'node:child_process';
import { readFile, lstat, realpath, writeFile } from 'node:fs/promises';
import { resolve, relative, isAbsolute, extname } from 'node:path';
import { fileURLToPath } from 'node:url';
const project = resolve(fileURLToPath(new URL('..', import.meta.url)));
const trackedOnly = process.argv.includes('--staged');
const args = trackedOnly ? ['ls-files', '-z'] : ['ls-files', '--cached', '--others', '--exclude-standard', '-z'];
const files = [...new Set(execFileSync('git', args, { cwd: project, encoding: 'utf8', windowsHide: true }).split('\0').filter(Boolean))];
const stagedBytes = new Map();
const stagedLinks = new Set();
if (trackedOnly && files.length) {
  if (files.some(file => /[\r\n]/.test(file))) throw new Error('Public source filenames cannot contain line breaks.');
  const batch = execFileSync('git', ['cat-file', '--batch'], { cwd: project, input: files.map(file => ':' + file).join('\n') + '\n', windowsHide: true, maxBuffer: 64 * 1024 * 1024 });
  let offset = 0;
  for (const file of files) {
    const end = batch.indexOf(10, offset);
    const header = batch.toString('utf8', offset, end);
    const size = / blob (\d+)$/.exec(header)?.[1];
    if (size === undefined) throw new Error('Could not read a staged source blob.');
    stagedBytes.set(file, batch.subarray(end + 1, end + 1 + Number(size)));
    offset = end + 1 + Number(size) + 1;
  }
  for (const entry of execFileSync('git', ['ls-files', '--stage', '-z'], { cwd: project, encoding: 'utf8', windowsHide: true }).split('\0')) {
    if (entry.startsWith('120000 ')) stagedLinks.add(entry.slice(entry.indexOf('\t') + 1));
  }
}
const escapedUser = (process.env.USERNAME || '').replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const patterns = [
  ['private-directory', /[A-Z]:[\\/]Users[\\/][^\\/\s"']+/i],
  ['private-directory', /\/(?:home|Users)\/[^/\s"']+\//],
  ['private-key', /-----BEGIN (?:[A-Z ]+)?PRIVATE KEY-----/],
  ['github-credential', /\b(?:gh[pousr]_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,})\b/],
  ['api-credential', /\bsk-[A-Za-z0-9_-]{32,}\b/],
];
if (escapedUser.length >= 3) patterns.push(['windows-username', new RegExp(escapedUser, 'i')]);
const findings = [];
for (const file of files) {
  const path = resolve(project, file);
  const local = relative(project, await realpath(path));
  if (stagedLinks.has(file) || local.startsWith('..') || isAbsolute(local) || (await lstat(path)).isSymbolicLink()) {
    findings.push({ file, kind: 'nonportable-link' }); continue;
  }
  if (/\.(?:exe|dll|asi|rpf|oiv|ytd|ydd|ydr|yft|ymap|ytyp|ybn|awc|msi|zip|7z|rar|key|pem|pfx|p12|dpapi|keystore|kdbx|db|sqlite\d*)(?:$|[-.])/i.test(file) || /(^|\/)\.env(?:\.|$)/.test(file)) {
    findings.push({ file, kind: 'private-or-generated-artifact' }); continue;
  }
  const bytes = trackedOnly ? stagedBytes.get(file) : await readFile(path);
  const texts = [bytes.toString('utf8'), bytes.toString('utf16le')];
  for (const [kind, pattern] of patterns) if (texts.some(text => pattern.test(text))) findings.push({ file, kind });
  if (!['.png', '.ico'].includes(extname(file)) && bytes.length > 5 * 1024 * 1024) findings.push({ file, kind: 'unexpected-large-source-file' });
}
const result = { passed: findings.length === 0, scope: trackedOnly ? 'index' : 'all public source candidates', files: files.length, findings };
const report = process.argv.indexOf('--report');
if (report >= 0) await writeFile(resolve(process.argv[report + 1]), JSON.stringify(result, null, 2) + '\n');
console.log(JSON.stringify(result));
if (!result.passed) process.exitCode = 1;
