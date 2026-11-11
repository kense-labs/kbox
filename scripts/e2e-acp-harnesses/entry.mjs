// Test-only bootstrap: consume credentials before handing stdin to the ACP harness.
import fs from 'node:fs';

const bytes = [], byte = Buffer.alloc(1);
while (fs.readSync(0, byte, 0, 1, null)) {
  if (byte[0] === 10) break;
  if (bytes.length >= 262144) throw new Error('oversized bootstrap');
  bytes.push(byte[0]);
}
const input = JSON.parse(Buffer.from(bytes).toString('utf8'));
const profile = process.argv[2];
process.env.HOME = '/data';
fs.mkdirSync('/data', {recursive: true});
fs.mkdirSync('/run/credentials', {recursive: true, mode: 0o700});
let executable, args;
switch (profile) {
  case 'aster': {
    if (!input.key || !input.baseUrl || !input.model) throw new Error('missing Aster provider');
    process.env.DEEPSEEK_API_KEY = input.key;
    const config = `default_model = ${JSON.stringify('deepseek/' + input.model)}
providers "deepseek" {
  base_url = ${JSON.stringify(input.baseUrl)}
  api_key = env("DEEPSEEK_API_KEY")
  models ${JSON.stringify(input.model)} { name = "Acceptance model" }
}
`;
    fs.writeFileSync('/run/credentials/aster.acl', config, {mode: 0o600});
    executable = '/opt/aster/aster-acp';
    args = [executable, '--cwd', '/workspace', '--config', '/run/credentials/aster.acl', '--reasoning-effort', 'low'];
    break;
  }
  case 'deepseek':
    if (!input.key) throw new Error('missing DeepSeek provider');
    process.env.DEEPSEEK_API_KEY = input.key;
    process.env.DSH_HOME = '/data';
    process.env.DSH_TELEMETRY_DISABLED = '1';
    executable = process.execPath;
    args = [executable, '/app/apps/cli/lib/bin.js', '--profile', 'acp'];
    break;
  case 'codex': {
    if (!input.auth || !input.model) throw new Error('missing Codex authentication');
    const home = '/run/credentials/codex';
    fs.mkdirSync(home, {mode: 0o700});
    fs.writeFileSync(`${home}/auth.json`, JSON.stringify(input.auth), {mode: 0o600});
    fs.writeFileSync(`${home}/config.toml`, `model = ${JSON.stringify(input.model)}
model_reasoning_effort = "low"
cli_auth_credentials_store = "file"
sqlite_home = "/data/sqlite"
`, {mode: 0o600});
    for (const name of ['sessions', 'archived_sessions']) {
      fs.mkdirSync(`/data/${name}`, {recursive: true});
      fs.symlinkSync(`/data/${name}`, `${home}/${name}`);
    }
    process.env.CODEX_HOME = home;
    process.env.CODEX_PATH = '/opt/codex/codex';
    executable = process.execPath;
    args = [executable, '/opt/codex-acp/dist/index.js'];
    break;
  }
  default:
    throw new Error('unknown harness profile');
}
process.execve(executable, args, process.env);
