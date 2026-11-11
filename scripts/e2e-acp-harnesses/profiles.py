"""Harness-specific launch/configuration, outside kbox's runtime implementation."""

import json
import os
from pathlib import Path
import re
import secrets

from acp_client import AcpClient
from client_services import ClientServices


class Profile:
    def __init__(self):
        self.name = os.environ['HARNESS_PROFILE']
        if self.name not in ('aster', 'deepseek', 'codex'):
            raise ValueError('unknown harness profile')
        self.redactions = []
        if self.name == 'codex':
            home = Path(os.environ.get('ACP_CODEX_HOME', str(Path.home() / '.codex')))
            auth = json.loads((home / 'auth.json').read_text())
            self.bootstrap = {'auth': auth, 'model': os.environ['ACP_CODEX_MODEL']}
            self.redactions = [v for v in auth.get('tokens', {}).values() if isinstance(v, str) and v]
            if auth.get('OPENAI_API_KEY'):
                self.redactions.append(auth['OPENAI_API_KEY'])
        else:
            config = Path(os.environ['ACP_PROVIDER_CONFIG']).read_text()
            provider = re.search(r'^providers\s+"deepseek"\s*\{(.*?)^\}', config, re.M | re.S)
            if provider is None:
                raise ValueError('missing exact deepseek provider block')
            values = {}
            for field in ('api_key', 'base_url'):
                match = re.search(r'^\s*' + field + r'\s*=\s*("(?:[^"\\]|\\.)*")\s*$', provider[1], re.M)
                if match is None:
                    raise ValueError(f'missing literal DeepSeek {field}')
                values[field] = json.loads(match[1])
            model = os.environ.get('DSH_MODEL', 'deepseek-flash')
            self.bootstrap = {'key': values['api_key'], 'baseUrl': values['base_url'], 'model': model}
            self.redactions = [values['api_key']]

    def client(self, mounts, evidence, label):
        box = f'acp-{self.name}-{label}-{secrets.token_hex(4)}'
        kbox = os.environ['KBOX']
        command = [kbox, 'run', '-i', '--rm', '--name', box, '--memory', '4g', '-u', 'root',
                   *mounts, '--tmpfs', '/run/credentials:size=16m', '--entrypoint', 'node',
                   '-v', f'{Path(__file__).parent.resolve()}:/acceptance', os.environ['IMAGE'], '--',
                   '/acceptance/entry.mjs', self.name]
        mounted = {}
        for value in mounts:
            if value.endswith(':/workspace') or value.endswith(':/external'):
                host, guest = value.rsplit(':', 1)
                mounted[guest] = Path(host)
        return HarnessClient(command, evidence, label, profile=self,
                             services=ClientServices(kbox, box, mounted))

    def restore(self, client, session):
        capabilities = client.initialized.get('agentCapabilities', {})
        client.services.sessions.add(session)
        if capabilities.get('loadSession'):
            method = 'session/load'
        elif 'resume' in capabilities.get('sessionCapabilities', {}):
            method = 'session/resume'
        else:
            raise AssertionError(f'{self.name} cannot restore a persisted session over ACP')
        before = len(client.updates)
        client.request(method, {'sessionId': session, 'cwd': '/workspace', 'mcpServers': []}, 180)
        replay = [u['update']['sessionUpdate'] for u in client.updates[before:] if u['sessionId'] == session]
        if method == 'session/load':
            assert 'user_message_chunk' in replay and 'agent_message_chunk' in replay, 'session/load omitted transcript replay'
        else:
            assert not {'user_message_chunk', 'agent_message_chunk', 'tool_call'}.intersection(replay), 'session/resume replayed history'
        return method

    def close_session(self, client, session):
        capabilities = client.initialized.get('agentCapabilities', {}).get('sessionCapabilities', {})
        if 'close' in capabilities:
            client.request('session/close', {'sessionId': session})

    def prepare_denial(self, client, session):
        if self.name in ('aster', 'codex'):
            client.request('session/set_mode', {'sessionId': session,
                           'modeId': 'default' if self.name == 'aster' else 'read-only'})


class HarnessClient(AcpClient):
    def __init__(self, *args, profile, services):
        self.profile, self.services = profile, services
        super().__init__(*args, bootstrap=profile.bootstrap, secrets=profile.redactions, client_handler=services)

    def initialize(self):
        self.initialized = super().initialize(ClientServices.capabilities)
        return self.initialized

    def new_session(self, cwd='/workspace'):
        session = super().new_session(cwd)
        self.services.sessions.add(session)
        return session

    def configure(self, session):
        if self.profile.name == 'deepseek':
            super().configure(session)
        elif self.profile.name == 'codex':
            self.request('session/set_mode', {'sessionId': session, 'modeId': 'workspace-write'})

    def close(self, strict=True):
        try:
            self.services.close()
        finally:
            super().close(strict)
