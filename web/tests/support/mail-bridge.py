"""Offline TLS IMAP fixture for real browser connection setup. No real mailbox."""
import socketserver, ssl, sys, threading

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
            if command=='CAPABILITY':answer='* CAPABILITY IMAP4rev1 STARTTLS\r\n'+tag+' OK capability'
            elif command=='STARTTLS':
                self.wfile.write((tag+' OK begin TLS\r\n').encode());self.wfile.flush()
                wrapped=context.wrap_socket(self.connection,server_side=True)
                self.connection=wrapped;self.rfile=wrapped.makefile('rb');self.wfile=wrapped.makefile('wb')
                continue
            elif command=='LOGIN':
                import shlex
                values=shlex.split(parts[2])
                authenticated=values==['fixture-local','fixture-imap-password']
                answer=tag+(' OK login' if authenticated else ' NO invalid local credentials')
            elif command=='LOGOUT':
                self.wfile.write(('* BYE fixture\r\n'+tag+' OK logout\r\n').encode());self.wfile.flush();return
            elif not authenticated:answer=tag+' NO login required'
            elif command=='EXAMINE':answer='* 0 EXISTS\r\n'+tag+' OK [READ-ONLY] mailbox'
            elif command=='STATUS':answer='* STATUS INBOX (UIDVALIDITY 42 UIDNEXT 1)\r\n'+tag+' OK status'
            else:answer=tag+' BAD unexpected fixture command'
            self.wfile.write((answer+'\r\n').encode());self.wfile.flush()
class Server(socketserver.ThreadingTCPServer):
    daemon_threads=True
    allow_reuse_address=True
with Server(('127.0.0.1',0),Handler) as server:
    print(server.server_address[1],flush=True)
    server.serve_forever(poll_interval=.1)
