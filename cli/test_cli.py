"""Isolated CLI integration checks; never contacts the user's systemd instance."""
import importlib.util
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('commut', Path(__file__).with_name('commut.py'))
cli = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cli)


class Commands(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='commut tests ')
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.old_env = os.environ.copy()
        self.addCleanup(lambda: (os.environ.clear(), os.environ.update(self.old_env)))
        os.environ['XDG_CONFIG_HOME'] = str(self.base / 'config space')
        os.environ['XDG_STATE_HOME'] = str(self.base / 'state space')
        self.bin = self.base / 'bin'
        self.bin.mkdir()
        os.environ['PATH'] = str(self.bin) + ':' + os.environ['PATH']
        os.environ['TEST_LOG'] = str(self.base / 'calls')
        for name, body in {
            'systemctl': 'echo "$*" >> "$TEST_LOG"\n[ "${2:-}" != status ] || exit 3\n[ "${FAIL_RELOAD:-}" != yes ] || [ "${2:-}" != daemon-reload ]',
            'loginctl': 'echo "$*" >> "$TEST_LOG"\nif [ "$1" = show-user ]; then echo "${TEST_LINGER:-yes}"; fi',
            'nix-store': 'ln -s "$5" "$2"',
        }.items():
            path = self.bin / name
            path.write_text('#!/bin/sh\n' + body + '\n')
            path.chmod(0o755)
        cli.SERVER = str(self.base / 'server')
        cli.CLIENT = str(self.base / 'client')
        cli.TEMPLATE = str(Path(__file__).resolve().parent.parent / 'installer/Caddyfile.template')
        self.target = self.base / 'sites space/site.caddy'
        self.key = self.base / 'public key.pem'
        self.key.write_text('public key')

    def command(self, *args):
        sys.argv = ['commut', *map(str, args)]
        cli.main()

    def test_sites(self):
        self.command('site', 'install', '--target', self.target, '--domain', 'http://localhost:8765')
        self.assertFalse((self.base / 'calls').read_text() if (self.base / 'calls').exists() else '')
        content = self.target.read_text()
        self.assertIn('/sockets/*', content)
        self.assertIn('/api/*', content)
        self.assertIn('"' + cli.CLIENT + '/build/app"', content)
        parent = self.base / 'Caddyfile'
        parent.write_text('{\n admin off\n auto_https off\n}\nimport "' + str(self.target) + '"\n')
        subprocess.run(['caddy', 'adapt', '--config', str(parent), '--adapter', 'caddyfile'], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        cli.CLIENT += '-new'
        self.command('site', 'install', '--target', self.target, '--domain', 'example.com')
        roots = list((self.base / 'state space/commut/gcroots').glob('site-*/*'))
        self.assertEqual(len(roots), 1)
        self.assertEqual(os.readlink(roots[0]), cli.CLIENT)
        self.command('site', 'uninstall', '--target', self.target)
        self.command('site', 'uninstall', '--target', self.target)
        self.target.write_text('unmanaged')
        with self.assertRaises(ValueError):
            self.command('site', 'install', '--target', self.target, '--domain', 'example.com')
        with self.assertRaises(ValueError):
            self.command('site', 'uninstall', '--target', self.target)
        self.assertEqual(self.target.read_text(), 'unmanaged')

    def test_live_routes(self):
        import http.client
        import http.server
        import socket
        import threading
        import time

        class Backend(http.server.BaseHTTPRequestHandler):
            def do_GET(self):
                if self.headers.get('Upgrade') == 'websocket':
                    self.send_response(101)
                    self.send_header('Connection', 'Upgrade')
                    self.send_header('Upgrade', 'websocket')
                    self.end_headers()
                else:
                    self.send_response(200)
                    self.end_headers()
                    self.wfile.write(self.path.encode())

            def log_message(self, *args):
                pass

        backend = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Backend)
        threading.Thread(target=backend.serve_forever, daemon=True).start()
        self.addCleanup(backend.server_close)
        self.addCleanup(backend.shutdown)
        for path, content in [('build/app/index.html', 'app'), ('public/favicon.ico', 'icon'), ('dist/main.js', 'js')]:
            asset = Path(cli.CLIENT) / path
            asset.parent.mkdir(parents=True, exist_ok=True)
            asset.write_text(content)
        with socket.socket() as sock:
            sock.bind(('127.0.0.1', 0))
            port = sock.getsockname()[1]
        self.command('site', 'install', '--target', self.target, '--domain', f'http://127.0.0.1:{port}', '--listen-port', backend.server_port)
        parent = self.base / 'Caddyfile'
        parent.write_text('{\n admin off\n auto_https off\n}\nimport "' + str(self.target) + '"\n')
        proc = subprocess.Popen(['caddy', 'run', '--config', str(parent), '--adapter', 'caddyfile'], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        try:
            for _ in range(100):
                try:
                    with socket.create_connection(('127.0.0.1', port), timeout=0.1):
                        break
                except OSError:
                    time.sleep(0.02)
            for path, expected in [('/app/index.html', b'app'), ('/favicon.ico', b'icon'), ('/dist/main.js', b'js'), ('/api/check', b'/api/check'), ('/sockets/check', b'/sockets/check')]:
                connection = http.client.HTTPConnection('127.0.0.1', port, timeout=3)
                connection.request('GET', path)
                response = connection.getresponse()
                self.assertEqual(response.status, 200)
                self.assertEqual(response.read(), expected)
                connection.close()
            connection = http.client.HTTPConnection('127.0.0.1', port, timeout=3)
            connection.request('GET', '/sockets/check', headers={'Connection': 'Upgrade', 'Upgrade': 'websocket'})
            self.assertEqual(connection.getresponse().status, 101)
            connection.close()
        finally:
            proc.terminate()
            proc.wait(timeout=5)

    def test_services(self):
        args = ('service', 'install', '--authorized-pubkey-file', self.key)
        self.command(*args)
        with self.assertRaises(ValueError):
            self.command(*args)
        unit = self.base / 'config space/systemd/user/commut.service'
        self.assertIn('/bin/commut-server', unit.read_text())
        for action in ('start', 'start', 'stop', 'stop', 'restart'):
            self.command('service', action)
        with self.assertRaises(SystemExit) as result:
            self.command('service', 'status')
        self.assertEqual(result.exception.code, 3)
        self.command('service', 'uninstall')
        self.command('service', 'uninstall')
        self.assertTrue(self.key.exists())
        self.assertFalse(unit.exists())
        calls = (self.base / 'calls').read_text()
        self.assertIn('--user disable --now commut.service', calls)
        self.assertNotIn('disable-linger', calls)
        os.environ['FAIL_RELOAD'] = 'yes'
        with self.assertRaises(subprocess.CalledProcessError):
            self.command(*args)
        self.assertFalse(unit.exists())
        self.assertFalse((self.base / 'state space/commut/gcroots/backend').exists())

    def test_linger_and_atomic_failure(self):
        os.environ['TEST_LINGER'] = 'no'
        with self.assertRaises(ValueError):
            self.command('service', 'install', '--authorized-pubkey-file', self.key)
        self.assertFalse((self.base / 'config space').exists())
        self.command('site', 'install', '--target', self.target, '--domain', 'example.com')
        previous = self.target.read_text()
        original = cli.atomic
        cli.atomic = lambda *args, **kwargs: (_ for _ in ()).throw(OSError('write failed'))
        self.addCleanup(setattr, cli, 'atomic', original)
        cli.CLIENT += '-failed'
        with self.assertRaises(OSError):
            self.command('site', 'install', '--target', self.target, '--domain', 'example.com')
        self.assertEqual(self.target.read_text(), previous)
        self.assertEqual(len(list((self.base / 'state space/commut/gcroots').glob('site-*/*'))), 1)

    def test_invalid(self):
        with self.assertRaises(ValueError):
            self.command('service', 'install', '--authorized-pubkey-file', self.base / 'missing')
        for args in [('site', 'install'), ('serve', '--unknown'), ('service', 'start', '--listen-port', '1'), ('site', 'install', '--target', self.target, '--domain', 'example.com', '--listen-port', '0')]:
            with self.assertRaises(SystemExit):
                self.command(*args)

    def test_serve_exec(self):
        server = Path(cli.SERVER) / 'bin/commut-server'
        server.parent.mkdir(parents=True)
        server.write_text('#!/bin/sh\n[ "$COMMUT_PORT" = 4567 ] || exit 1\nexit 37\n')
        server.chmod(0o755)
        driver = self.base / 'driver.py'
        driver.write_text(Path(__file__).with_name('commut.py').read_text().replace("'@server@'", repr(cli.SERVER)))
        env = dict(os.environ, COMMUT_PORT='4567')
        result = subprocess.run([sys.executable, str(driver), 'serve'], env=env)
        self.assertEqual(result.returncode, 37)
        server.write_text('#!/bin/sh\nexec sleep 60\n')
        proc = subprocess.Popen([sys.executable, str(driver), 'serve'], env=env)
        try:
            import time
            time.sleep(0.2)
            proc.send_signal(signal.SIGTERM)
            self.assertEqual(proc.wait(timeout=5), -signal.SIGTERM)
        finally:
            if proc.poll() is None:
                proc.kill()


if __name__ == '__main__':
    unittest.main()
