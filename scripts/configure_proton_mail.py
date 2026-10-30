#!/usr/bin/env python3
"""Operator-only credential provisioning. Run locally on the Collector host."""
import argparse
import getpass
import hashlib
import json
import os
from pathlib import Path
import ssl
import uuid
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--output',required=True,type=Path)
p.add_argument('--certificate',required=True,type=Path,help='cert.pem exported locally by Bridge CLI')
p.add_argument('--owner',required=True,type=uuid.UUID,help='Collector user_id from /api/v1/me')
p.add_argument('--id',default='proton-owner')
p.add_argument('--folder',action='append',dest='folders',help='Exact IMAP folder name; repeat as needed; default INBOX')
a=p.parse_args()
if not a.id or len(a.id)>64 or not all(c.isascii() and (c.isalnum() or c in '-_') for c in a.id):p.error('Invalid profile id')
certificate=ssl.PEM_cert_to_DER_cert(a.certificate.read_text())
username=getpass.getpass('Local Bridge IMAP username (hidden): ')
password=getpass.getpass('Local Bridge IMAP password (hidden): ')
if not username or not password:p.error('Bridge username and password are required')
record=[dict(id=a.id,owner=str(a.owner),label='My Proton Mail',username=username,password=password,certificate_sha256=hashlib.sha256(certificate).hexdigest(),port=1143,folders=a.folders or ['INBOX'])]
fd=os.open(a.output,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
with os.fdopen(fd,'w') as f:
    json.dump(record,f);f.write('\n');f.flush();os.fsync(f.fileno())
print('Private profile created. Assign it to Collector UID 10001, mount it read-only, and restart the candidate server. No credentials were printed.')
