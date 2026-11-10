"""Offline TLS IMAP fixture for real browser connection setup. No real mailbox."""
import socketserver, ssl, sys, json
from pathlib import Path
def controls():
    return json.loads(Path(sys.argv[4]).read_text()) if len(sys.argv)>4 else {}

context=ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
context.load_cert_chain(sys.argv[1],sys.argv[2])
class Handler(socketserver.StreamRequestHandler):
    def handle(self):
        self.connection.settimeout(10)
        self.wfile.write(b'* OK Disposable mail fixture\r\n');self.wfile.flush()
        authenticated=False
        while True:
            line=self.rfile.readline(4097)
            if not line or len(line)>4096:return
            parts=line.decode('ascii').strip().split(' ',2)
            if len(parts)<2:return
            tag,command=parts[:2];command=command.upper()
            if len(sys.argv)>3:
                with open(sys.argv[3],'a') as log:
                    if command in ('EXAMINE','LIST'):log.write(command+' '+parts[2]+'\n')
            if command=='CAPABILITY':answer='* CAPABILITY IMAP4rev1 STARTTLS\r\n'+tag+' OK capability'
            elif command=='STARTTLS':
                self.wfile.write((tag+' OK begin TLS\r\n').encode());self.wfile.flush()
                wrapped=context.wrap_socket(self.connection,server_side=True)
                self.connection=wrapped;self.rfile=wrapped.makefile('rb');self.wfile=wrapped.makefile('wb')
                continue
            elif command=='LOGIN':
                import shlex
                values=shlex.split(parts[2])
                authenticated=values==['fixture-local',controls().get('password','fixture-imap-password')]
                answer=tag+(' OK login' if authenticated else ' NO invalid local credentials')
            elif command=='LOGOUT':
                self.wfile.write(('* BYE fixture\r\n'+tag+' OK logout\r\n').encode());self.wfile.flush();return
            elif not authenticated:answer=tag+' NO login required'
            elif command=='LIST':
                answer=('* LIST () "/" INBOX\r\n'
                        '* LIST (\\Noselect) "/" "Folders"\r\n'
                        '* LIST () "/" "Folders/Receipts"\r\n'
                        '* LIST () "/" {26}\r\nFolders/&ZeVnLIqe- &- Shop\r\n'
                        +tag+' OK list')
            elif command=='EXAMINE':answer='* 0 EXISTS\r\n'+tag+' OK [READ-ONLY] mailbox'
            elif command=='STATUS':answer=f"* STATUS INBOX (UIDVALIDITY 42 UIDNEXT {controls().get('uidnext',1)})\r\n"+tag+' OK status'
            elif command=='UID' and parts[2].upper().startswith('SEARCH '):
                first,last=map(int,parts[2].split()[-1].split(':'))
                uid=' 1' if controls().get('amazon_receipt') and first<=1<=last else ''
                answer=f'* SEARCH{uid}\r\n{tag} OK search'
            elif command=='UID' and parts[2].startswith('FETCH 1 ') and controls().get('amazon_receipt'):
                raw=Path(sys.argv[5]).read_bytes()
                if 'BODY.PEEK[HEADER.FIELDS ' in parts[2]:
                    data=raw.split(b'\r\n\r\n',1)[0]+b'\r\n\r\n'
                    field=f'RFC822.SIZE {len(raw)} BODY[HEADER]'
                elif 'BODY.PEEK[]' in parts[2]:
                    data=raw
                    field='BODY[]'
                else:
                    return
                self.wfile.write(f'* 1 FETCH (UID 1 {field} {{{len(data)}}}\r\n'.encode()+data+b')\r\n')
                answer=tag+' OK fetch'
            else:answer=tag+' BAD unexpected fixture command'
            self.wfile.write((answer+'\r\n').encode());self.wfile.flush()
class Server(socketserver.ThreadingTCPServer):
    daemon_threads=True
    allow_reuse_address=True
with Server(('127.0.0.1',0),Handler) as server:
    print(server.server_address[1],flush=True)
    server.serve_forever(poll_interval=.1)
