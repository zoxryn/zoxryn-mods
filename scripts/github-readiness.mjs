// Read-only access checks. Never pushes or displays credential helper output.
import { spawnSync } from 'node:child_process';
import { writeFile } from 'node:fs/promises';
const repository = 'zoxryn/zoxryn-mods';
const result = { repository, accessible: false, empty: null, authenticated: false, pushPermissionReported: null, published: false };
const env = { ...process.env, GIT_TERMINAL_PROMPT: '0', GCM_INTERACTIVE: 'Never', GCM_GUI_PROMPT: '0' };
let token = process.env.GH_TOKEN || process.env.GITHUB_TOKEN || '';
if (!token) {
  const helpers = spawnSync('git', ['config', '--get-all', 'credential.helper'], { env, windowsHide: true, encoding: 'utf8', timeout: 8000 });
  if (helpers.status === 0 && helpers.stdout.trim()) {
    const credentials = spawnSync('git', ['-c', 'credential.interactive=never', 'credential', 'fill'], {
      input: 'protocol=https\nhost=github.com\n\n', env, windowsHide: true, encoding: 'utf8', timeout: 8000,
    });
    if (credentials.status === 0) token = credentials.stdout.split(/\r?\n/).find(line => line.startsWith('password='))?.slice(9) || '';
  }
}
const headers = { accept: 'application/vnd.github+json', 'user-agent': 'Zoxryn-Mods-release-readiness', ...(token ? { authorization: 'Bearer ' + token } : {}) };
const response = await fetch('https://api.github.com/repos/' + repository, { headers, redirect: 'error', signal: AbortSignal.timeout(15000) });
if (response.ok) {
  const repo = await response.json(); result.accessible = true; result.empty = repo.size === 0;
  result.authenticated = Boolean(token && repo.permissions); result.pushPermissionReported = repo.permissions?.push ?? null;
  result.visibility = repo.visibility; result.defaultBranch = repo.default_branch;
} else result.httpStatus = response.status;
token = ''; delete headers.authorization;
if (!result.authenticated) result.authenticationNote = 'Authenticated push permission was not established. No login prompt, push or publication was attempted.';
const report = process.argv[2];
if (report) await writeFile(report, JSON.stringify(result, null, 2) + '\n');
console.log(JSON.stringify(result));
