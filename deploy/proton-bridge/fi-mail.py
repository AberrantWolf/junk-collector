"""Fi operator helper. Credentials are entered only in the private terminal."""
import argparse, json, os, sqlite3, subprocess, sys
from pathlib import Path

os.umask(0o077)
deploy=Path('/opt/junk-collector-beta14/deploy/container')
mail=Path('/srv/junk-collector/mail')
compose=['docker','compose','--env-file','/etc/junk-collector/remote.env']
for name in ['compose.yaml','compose.media.yaml','compose.inbox.yaml','compose.remote.yaml','compose.control.yaml','compose.mail.yaml']:
    compose+=['-f',str(deploy/name)]
p=argparse.ArgumentParser(description=__doc__)
s=p.add_subparsers(dest='command',required=True)
s.add_parser('login',help='Stop only Bridge and open its private interactive CLI; restart Bridge on exit')
s.add_parser('accounts',help='List enabled Collector account IDs for --owner')
c=s.add_parser('configure',help='Pin the exported certificate and enter local IMAP credentials with hidden prompts')
c.add_argument('--owner',required=True)
c.add_argument('--folder',action='append',default=[])
s.add_parser('status',help='Show stack state without printing credentials or mail')
a=p.parse_args()
if a.command in ('login','configure') and not sys.stdin.isatty():sys.exit('Use a private terminal: ssh -t fi-home /home/link/bin/collector-mail '+a.command)
if a.command=='login':
    subprocess.run(compose+['stop','proton-bridge'],check=True)
    try:
        print('At the Bridge prompt: login; telemetry disable; updates autoupdates disable; cert export (directory /home/bridge/export); info. Then exit.')
        print('Enter account passwords/2FA only here. Use the distinct local IMAP credentials for configure.')
        subprocess.run(compose+['run','--rm','--no-deps','proton-bridge','cli'],check=True)
    finally:
        subprocess.run(compose+['up','-d','--no-deps','--pull','never','proton-bridge'],check=True)
elif a.command=='accounts':
    db=sqlite3.connect('file:/srv/junk-collector/state/private.sqlite?mode=ro',uri=True)
    for uid,name in db.execute('SELECT id,username FROM users WHERE enabled=1 ORDER BY username'):
        print(uid,name)
elif a.command=='configure':
    if any(Path('/srv/junk-collector/state/mail-profiles').glob('*.json')):sys.exit('A web-managed mail profile exists. Use Revalidate Bridge connection in Purchase inbox to repair credentials or the certificate pin.')
    pending=mail/'profiles.pending.json'
    if pending.exists():sys.exit('A pending private profile already exists; preserve/review it before retrying.')
    command=[sys.executable,str(deploy.parent.parent/'scripts/configure_proton_mail.py'),'--output',str(pending),'--certificate',str(mail/'bridge-home/export/cert.pem'),'--owner',a.owner]
    for folder in a.folder:command+=['--folder',folder]
    subprocess.run(command,check=True)
    os.chown(pending,10001,10001)
    profile=mail/'profiles.json'
    if json.loads(profile.read_text()):
        pending.unlink();sys.exit('A profile is already configured; rotation needs a retained private backup.')
    os.replace(pending,profile)
    subprocess.run(['systemctl','restart','junk-collector-remote.service'],check=True)
    print('Profile installed. Open the private Purchase inbox, select Games and enable My Proton Mail.')
elif a.command=='status':
    result=subprocess.run([sys.executable,str(deploy.parent.parent/'scripts/bridge_status.py'),str(mail/'status/status.json')])
    subprocess.run(compose+['ps'],check=True)
    raise SystemExit(result.returncode)
