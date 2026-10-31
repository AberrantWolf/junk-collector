#!/usr/bin/env python3
"""Synthetic, offline receipt/IMAP tests. No account, mailbox or retailer requests."""
import importlib.util
import json
import re
from pathlib import Path
import unittest
from email.message import EmailMessage
from unittest.mock import patch
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('mail',Path(__file__).resolve().parents[1]/'crates/collector-providers/src/mail/connector.py')
mail=importlib.util.module_from_spec(spec);spec.loader.exec_module(mail)
def receipt(sender,subject,body,html=False,mid='receipt-1',charset='utf-8'):
    m=EmailMessage();m['From']=sender;m['Subject']=subject;m['Message-ID']=f'<{mid}@fixture.invalid>';m['Date']='Fri, 30 Oct 2026 10:00:00 +0900'
    m.set_content(body,subtype='html' if html else 'plain',charset=charset);return m.as_bytes()
class Receipts(unittest.TestCase):
    def test_five_retailers_and_japanese_encoding(self):
        cases=[('a@amazon.co.jp','ご注文の確認','注文番号: 123-1234567-1234567\n商品名: ゼルダ Switch 2','amazon_jp','ordered'),('a@joshin.co.jp','ご予約の確認','ご注文番号: J12345\n商品名: ゼルダ コレクターズ Switch','joshin','preordered'),('a@square-enix.com','商品出荷のお知らせ','受注番号: E12345\n商品名: FINAL FANTASY PS5','square_enix','shipped'),('a@accounts.nintendo.com','ご購入明細','注文番号: N12345\nソフト名: マリオカート','nintendo','purchased'),('a@steampowered.com','Steam purchase receipt','Transaction ID: 12345678\nItem: Hades II','steam','purchased')]
        for sender,subject,body,retailer,status in cases:
            with self.subTest(retailer=retailer):
                result=mail.extract(receipt(sender,subject,body,charset='iso-2022-jp'),'private-profile')
                self.assertEqual(len(result),1);self.assertEqual(result[0]['retailer'],retailer);self.assertEqual(result[0]['status'],status);self.assertEqual(len(result[0]['source_keys'][0]),64)
    def test_html_multi_item_dedup(self):
        raw=receipt('a@steampowered.com','Steam purchase receipt','<p>Transaction ID: ABC12345</p><a href="https://store.steampowered.com/app/1/">Hades</a><a href="https://store.steampowered.com/app/1/">Hades</a><td class="receipt_item_name">Portal 2</td><img src="https://tracker.invalid/pixel">',True)
        result=mail.extract(raw,'p');self.assertEqual([p['title'] for p in result],['Hades','Portal 2']);self.assertEqual(result,mail.extract(raw,'p'));self.assertNotEqual(result[0]['source_keys'],mail.extract(raw,'other')[0]['source_keys'])
    def test_footer_does_not_cancel_order(self):
        self.assertEqual(mail.extract(receipt('a@amazon.co.jp','ご注文の確認','商品名: Game\n返金 キャンセルの方法'),'p')[0]['status'],'ordered')
        self.assertEqual(mail.extract(receipt('a@steampowered.com','Your refund receipt','Item: Game'),'p')[0]['status'],'refunded')
    def test_unknown_and_spoofed_sender(self):
        self.assertEqual(mail.extract(receipt('a@joshin.co.jp','ご注文の確認','unknown layout'),'p')[0]['status'],'unknown')
        for sender in ['a@joshin.co.jp.attacker.invalid','Joshin <a@attacker.invalid>']:
            self.assertEqual(mail.extract(receipt(sender,'ご注文の確認','商品名: Game'),'p'),[])
        self.assertEqual(mail.extract(receipt('a@joshin.co.jp','認証コード','商品名: Secret'),'p'),[])
    def test_rfc3501_japanese_folder_vector(self):
        self.assertEqual(mail.imap_folder('日本語'), '&ZeVnLIqe-')
        self.assertEqual(mail.imap_folder('Inbox & Orders'), 'Inbox &- Orders')
    def test_privacy_and_bounds(self):
        encoded=json.dumps(mail.extract(receipt('a@joshin.co.jp','ご注文の確認','商品名: Game\n住所: PRIVATE ADDRESS\n決済: SECRET CARD'),'p'))
        for secret in ['PRIVATE ADDRESS','SECRET CARD','receipt-1','joshin.co.jp']:self.assertNotIn(secret,encoded)
        with self.assertRaises(ValueError):mail.extract(b'x'*(mail.MAX_MAIL+1),'p')
class FakeIMAP:
    commands=[]
    def __init__(self,*args,**kwargs):self.sock=self
    def getpeercert(self,**kwargs):return b'certificate'
    def starttls(self,context):self.commands.append('STARTTLS');return 'OK',[]
    def login(self,*args):self.commands.append('LOGIN');return 'OK',[]
    def select(self,folder,readonly=False):
        assert readonly;self.commands.append('EXAMINE');return 'OK',[b'1']
    def status(self,*args):return 'OK',[b'INBOX (UIDVALIDITY 42 UIDNEXT 2)']
    def uid(self,command,*args):
        self.commands.append((command,args))
        if command=='search':return 'OK',[b'1']
        raw=receipt('a@joshin.co.jp','ご予約の確認','注文番号: J12345\n商品名: Game')
        if 'HEADER' in args[-1]:return 'OK',[(f'1 (UID 1 RFC822.SIZE {len(raw)} BODY[HEADER]'.encode(),raw)]
        assert 'BODY.PEEK[]' in args[-1];return 'OK',[(b'1 BODY[]',raw)]
    def logout(self):self.commands.append('LOGOUT')
class IMAP(unittest.TestCase):
    def test_readonly_history_resume_and_reset(self):
        FakeIMAP.commands=[];profile=dict(id='private',username='local',password='secret',certificate_sha256=mail.hashlib.sha256(b'certificate').hexdigest())
        with patch.object(mail.imaplib,'IMAP4',FakeIMAP):
            result=mail.fetch_batch(profile,'');self.assertEqual(len(result['purchases']),1);self.assertEqual(json.loads(result['cursor'])['INBOX'],dict(validity=42,uid=1))
            self.assertEqual(mail.fetch_batch(profile,result['cursor'])['purchases'],[])
            self.assertEqual(mail.fetch_batch(profile,json.dumps({'INBOX':{'validity':1,'uid':999}}))['purchases'],result['purchases'])
        self.assertLess(FakeIMAP.commands.index('STARTTLS'),FakeIMAP.commands.index('LOGIN'));self.assertNotIn('STORE',str(FakeIMAP.commands));self.assertNotIn('EXPUNGE',str(FakeIMAP.commands))
    def test_added_folders_start_history_without_restarting_existing_folders(self):
        selected=[]
        class FoldersIMAP(FakeIMAP):
            def select(self,folder,readonly=False):
                selected.append(folder)
                return super().select(folder,readonly)
        profile=dict(id='private',username='local',password='secret',certificate_sha256=mail.hashlib.sha256(b'certificate').hexdigest(),folders=['INBOX','Folders/Receipts'])
        cursor=json.dumps({'INBOX':dict(validity=42,uid=1)})
        FakeIMAP.commands=[]
        with patch.object(mail.imaplib,'IMAP4',FoldersIMAP):
            result=mail.fetch_batch(profile,cursor)
            self.assertCountEqual(selected,['"INBOX"','"Folders/Receipts"'])
            self.assertEqual(len(result['purchases']),1)
            self.assertEqual(json.loads(result['cursor']),{'INBOX':dict(validity=42,uid=1),'Folders/Receipts':dict(validity=42,uid=1)})
            self.assertEqual(sum(isinstance(c,tuple) and c[0]=='search' for c in FakeIMAP.commands),1)
            selected.clear()
            profile['folders']=['Folders/Receipts']
            removed=mail.fetch_batch(profile,result['cursor'])
            self.assertEqual(selected,['"Folders/Receipts"'])
            self.assertEqual(removed['purchases'],[])
            self.assertEqual(json.loads(removed['cursor']),json.loads(result['cursor']))
            profile['folders']=['INBOX','Folders/Receipts']
            self.assertEqual(mail.fetch_batch(profile,removed['cursor'])['purchases'],[])

    def test_malformed_receipt_does_not_poison_the_next_message(self):
        bad=re.sub(rb'(?m)^Date:[^\r\n]*',b'Date: not-a-date',receipt('a@joshin.co.jp','ご予約の確認','注文番号: J12345\n商品名: Bad date'))
        good=receipt('a@joshin.co.jp','注文の確認','注文番号: J23456\n商品名: Valid later receipt')
        class Mixed(FakeIMAP):
            def status(self,*args):return 'OK',[b'INBOX (UIDVALIDITY 42 UIDNEXT 3)']
            def uid(self,command,*args):
                if command=='search':return 'OK',[b'1 2']
                raw=bad if args[0]==b'1' else good
                if 'HEADER' in args[-1]:return 'OK',[(f'1 (RFC822.SIZE {len(raw)} INTERNALDATE "31-Oct-2026 10:00:00 +0000" BODY[HEADER]'.encode(),raw)]
                return 'OK',[(b'1 BODY[]',raw)]
        profile=dict(id='p',username='u',password='p',certificate_sha256=mail.hashlib.sha256(b'certificate').hexdigest())
        with patch.object(mail.imaplib,'IMAP4',Mixed):result=mail.fetch_batch(profile,'')
        self.assertEqual(len(result['purchases']),2)
        self.assertEqual(result['details'][0]['issue'],'malformed_date')
        self.assertEqual(result['details'][0]['source']['uid'],1)
        self.assertEqual(result['details'][1]['source']['subject'],'注文の確認')
        self.assertEqual(result['purchases'][1]['title'],'Valid later receipt')
        self.assertEqual(result['purchases'][0]['observed_at'],'2026-10-31T10:00:00+00:00')
        self.assertEqual(json.loads(result['cursor'])['INBOX']['uid'],2)
        self.assertTrue(result['progress'][0]['caught_up'])

    def test_excessive_mime_is_reviewable_and_incomplete_bytes_still_block(self):
        from email.message import EmailMessage
        message=EmailMessage();message['From']='a@joshin.co.jp';message['Subject']='注文の確認';message['Message-ID']='<mime-limit>'
        message.make_mixed()
        for _ in range(101):
            part=EmailMessage();part.set_content('商品名: Game');message.attach(part)
        result=mail.extract(message.as_bytes(),'p')
        self.assertEqual(result[0]['_issue'],'too_many_parts')
        self.assertEqual(result[0]['status'],'unknown')
        class Incomplete(FakeIMAP):
            def uid(self,command,*args):
                result=super().uid(command,*args)
                if command=='fetch' and 'HEADER' not in args[-1]:return 'OK',[(b'1 BODY[]',b'truncated')]
                return result
        profile=dict(id='p',username='u',password='p',certificate_sha256=mail.hashlib.sha256(b'certificate').hexdigest())
        with patch.object(mail.imaplib,'IMAP4',Incomplete):
            with self.assertRaises(ValueError):mail.fetch_batch(profile,'')
        FakeIMAP.commands=[]
        with patch.object(mail.imaplib,'IMAP4',FakeIMAP):mail.fetch_batch(profile,'',auth_only=True)
        self.assertNotIn('EXAMINE',FakeIMAP.commands)

    def test_deep_mime_without_message_id_is_bounded_and_has_a_stable_raw_identity(self):
        depth=1100
        raw=b'From: a@joshin.co.jp\r\nSubject: Order receipt\r\nDate: Fri, 30 Oct 2026 10:00:00 +0000\r\nContent-Type: multipart/mixed; boundary=b0\r\n\r\n'
        for i in range(depth):raw+=f'--b{i}\r\nContent-Type: multipart/mixed; boundary=b{i+1}\r\n\r\n'.encode()
        raw+=f'--b{depth}\r\nContent-Type: text/plain\r\n\r\nItem: Game\r\n--b{depth}--\r\n'.encode()
        for i in reversed(range(depth)):raw+=f'--b{i}--\r\n'.encode()
        result=mail.extract(raw,'p')
        self.assertEqual(len(result),1)
        self.assertEqual(result[0]['_issue'],'too_many_parts')
        self.assertEqual(result[0]['status'],'unknown')
        identity=mail.hashlib.sha256(raw).hexdigest()
        expected=mail.hashlib.sha256(('p\0'+identity+'\0Unparsed receipt — review original email').encode()).hexdigest()
        self.assertEqual(result[0]['id'],expected)
        self.assertNotEqual(result[0]['id'],mail.extract(raw.replace(b'Item: Game',b'Item: Other'),'p')[0]['id'])
        ordinary=re.sub(rb'(?m)^Message-ID:[^\r\n]*\r?\n',b'',receipt('a@joshin.co.jp','Order receipt','商品名: Game'))
        canonical=mail.BytesParser(policy=mail.email.policy.default).parsebytes(ordinary).as_bytes()
        expected=mail.hashlib.sha256(('p\0'+mail.hashlib.sha256(canonical).hexdigest()+'\0Game').encode()).hexdigest()
        self.assertEqual(mail.extract(ordinary,'p')[0]['id'],expected)

    def test_refusal_categories_do_not_become_raw_imap_status_values(self):
        class Refused(FakeIMAP):
            def select(self,*args,**kwargs):return 'NO',[b'private provider response']
        profile=dict(id='p',username='u',password='p',certificate_sha256=mail.hashlib.sha256(b'certificate').hexdigest(),folders=['Archive'])
        with patch.object(mail.imaplib,'IMAP4',Refused):
            with self.assertRaises(mail.MailError) as failure:mail.fetch_batch(profile,'')
        self.assertEqual(failure.exception.kind,'folder')
        self.assertEqual(failure.exception.folder,'Archive')
        self.assertNotIn('private provider response',str(failure.exception))
        with self.assertRaises(mail.MailError) as failure:mail.checked(('NO',[]),'authentication')
        self.assertEqual(failure.exception.kind,'authentication')

    def test_pin_mismatch_never_sends_credentials(self):
        FakeIMAP.commands=[]
        with patch.object(mail.imaplib,'IMAP4',FakeIMAP):
            with self.assertRaises(ValueError):mail.fetch_batch(dict(id='p',username='u',password='p',certificate_sha256='0'*64),'')
        self.assertNotIn('LOGIN',FakeIMAP.commands)
    def test_connection_probe_never_fetches_messages(self):
        FakeIMAP.commands=[]
        profile=dict(id='p',username='local',password='secret',certificate_sha256=mail.hashlib.sha256(b'certificate').hexdigest())
        with patch.object(mail.imaplib,'IMAP4',FakeIMAP):
            result=mail.fetch_batch(profile,'',probe=True)
        self.assertEqual(result['purchases'],[])
        self.assertIn('EXAMINE',FakeIMAP.commands)
        self.assertFalse(any(isinstance(c,tuple) for c in FakeIMAP.commands))
    def test_local_credentials_cannot_inject_imap_commands(self):
        FakeIMAP.commands=[]
        with patch.object(mail.imaplib,'IMAP4',FakeIMAP):
            for field in ('username','password'):
                profile=dict(username='local',password='secret');profile[field]='bad\r\nA STORE 1 +FLAGS (\\Seen)'
                with self.assertRaises(ValueError):mail.fetch_batch(profile,'',probe=True)
        self.assertEqual(FakeIMAP.commands,[])

class BridgeReporter(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        import importlib.util
        spec=importlib.util.spec_from_file_location('bridge_status',ROOT/'deploy/proton-bridge/status.py')
        cls.reporter=importlib.util.module_from_spec(spec);spec.loader.exec_module(cls.reporter)
    def test_real_event_vectors_and_no_identifiers_or_invented_completion(self):
        s=self.reporter.Status()
        s.consume('>>> A sync has begun for private-account.',10)
        s.consume('Sync (private-account): 37.5% (Elapsed: 12.0s, ETA: 20.0s)',11)
        result=s.snapshot(12)
        self.assertEqual((result['phase'],result['percent'],result['remaining_seconds']),('syncing',37.5,20))
        self.assertNotIn('private-account',json.dumps(result))
        s.consume('Sync (private-account): 100.0% (Elapsed: 25.0s, ETA: 0.0s)',13)
        self.assertEqual(s.snapshot(14)['phase'],'syncing')
        s.consume('A sync has finished for private-account.',15)
        self.assertEqual(s.snapshot(16)['phase'],'synced');self.assertIsNone(s.snapshot(16)['percent'])
        s.consume('ERRO[time] Sync aborted error="context canceled" user="private-id"',17)
        self.assertEqual(s.snapshot(18)['phase'],'interrupted');self.assertIsNone(s.snapshot(18)['percent'])
    def test_multiple_accounts_and_invalid_vectors_never_invent_an_average(self):
        s=self.reporter.Status()
        for name in ['first','second']:s.consume(f'Sync ({name}): 10.0% (Elapsed: 2.0s, ETA: 18.0s)',1)
        self.assertIsNone(s.snapshot(2)['percent'])
        s.consume('Sync (third): 101.0% (Elapsed: 1.0s, ETA: 2.0s)',3)
        self.assertEqual(len(s.accounts),2)
        s.consume('A sync has finished for first.',4)
        self.assertEqual(s.snapshot(5)['phase'],'syncing')
        s.consume('ERRO[t] Sync aborted error="context canceled" user="opaque"',6)
        s.consume('A sync has finished for second.',7)
        self.assertEqual(s.snapshot(8)['phase'],'unknown')
        self.assertIsNone(s.snapshot(8)['percent'])
    def test_process_shutdown_reaps_child_and_clears_progress(self):
        import os,signal,subprocess,sys,tempfile,time
        from pathlib import Path
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);(root/'home').mkdir();(root/'status').mkdir()
            fake=root/'fake.py';fake.write_text('import sys\nprint("x"*100000,flush=True)\nprint("PRIVATE MESSAGE CONTENT",flush=True)\nprint("Sync (secret-account): 37.5% (Elapsed: 12.0s, ETA: 20.0s)",flush=True)\nfor line in sys.stdin:\n if line.strip()=="exit":\n  print("PRIVATE SHUTDOWN"*20000,flush=True);break\n')
            code=f'import importlib.util; s=importlib.util.spec_from_file_location("s",{str(ROOT/"deploy/proton-bridge/status.py")!r});m=importlib.util.module_from_spec(s);s.loader.exec_module(m);raise SystemExit(m.main([{sys.executable!r},{str(fake)!r}]))'
            child=subprocess.Popen([sys.executable,'-c',code],env=dict(os.environ,HOME=str(root/'home'),BRIDGE_STATUS_DIRECTORY=str(root/'status')),stdout=subprocess.PIPE,stderr=subprocess.PIPE)
            try:
                deadline=time.monotonic()+12
                while time.monotonic()<deadline:
                    try:
                        result=json.loads((root/'status/status.json').read_text())
                        if result['percent']==37.5:break
                    except (OSError,ValueError):pass
                    time.sleep(.05)
                else:self.fail('Reporter did not consume bounded fake-process progress')
                child.send_signal(signal.SIGTERM);out,err=child.communicate(timeout=5)
                self.assertEqual(child.returncode,0,err)
                self.assertEqual(out,b'')
                result=json.loads((root/'status/status.json').read_text())
                self.assertEqual(result['phase'],'stopped');self.assertIsNone(result['percent'])
                self.assertNotIn('secret-account',json.dumps(result))
            finally:
                if child.poll() is None:child.kill();child.communicate(timeout=5)
    def test_terminal_query_does_not_show_stale_progress_or_raw_errors(self):
        import tempfile
        spec=importlib.util.spec_from_file_location('query_status',ROOT/'scripts/bridge_status.py')
        query=importlib.util.module_from_spec(spec);spec.loader.exec_module(query)
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'status.json';value=self.reporter.Status().snapshot(100)
            value.update(phase='syncing',percent=37.5,event_at=100)
            p.write_text(json.dumps(value));text,code=query.query(p,105)
            self.assertEqual(code,0);self.assertIn('37.5%',text)
            text,code=query.query(p,121);self.assertEqual(code,2);self.assertNotIn('37.5%',text)
            p.write_text('PRIVATE malformed output');text,code=query.query(p,105)
            self.assertEqual(code,2);self.assertNotIn('PRIVATE',text)
            for candidate in [dict(value,event_at=None,phase='synced',percent=None),dict(value,percent=True),dict(value,schema=True),dict(value,percent=10**1000)]:
                p.write_text(json.dumps(candidate));text,code=query.query(p,105)
                self.assertEqual(code,2);self.assertNotIn('37.5%',text)
    def test_terminal_query_refuses_a_pipe_without_waiting_for_a_writer(self):
        import os,subprocess,sys,tempfile
        with tempfile.TemporaryDirectory() as d:
            fifo=Path(d)/'status.json';os.mkfifo(fifo)
            result=subprocess.run([sys.executable,str(ROOT/'scripts/bridge_status.py'),str(fifo)],capture_output=True,text=True,timeout=2)
            self.assertEqual(result.returncode,2);self.assertIn('unavailable',result.stdout)
    def test_forced_stop_is_not_clean_and_reaps_the_actual_process(self):
        import os,signal,subprocess,sys,tempfile,time
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);(root/'home').mkdir();(root/'status').mkdir();pid=root/'child.pid'
            fake=root/'stuck.py';fake.write_text(f'import os,signal,time\nfrom pathlib import Path\nsignal.signal(signal.SIGTERM,signal.SIG_IGN)\nPath({str(pid)!r}).write_text(str(os.getpid()))\nwhile True:time.sleep(1)\n')
            code=f'import importlib.util; s=importlib.util.spec_from_file_location("s",{str(ROOT/"deploy/proton-bridge/status.py")!r});m=importlib.util.module_from_spec(s);s.loader.exec_module(m);raise SystemExit(m.main([{sys.executable!r},{str(fake)!r}],shutdown_timeout=.1,reap_timeout=1))'
            child=subprocess.Popen([sys.executable,'-c',code],env=dict(os.environ,HOME=str(root/'home'),BRIDGE_STATUS_DIRECTORY=str(root/'status')),stdout=subprocess.PIPE,stderr=subprocess.PIPE)
            try:
                deadline=time.monotonic()+5
                while not pid.exists() and time.monotonic()<deadline:time.sleep(.02)
                self.assertTrue(pid.exists(),'Owned fake process did not start')
                child.send_signal(signal.SIGTERM);out,err=child.communicate(timeout=5)
                self.assertEqual(child.returncode,1,err);self.assertEqual(out,b'')
                with self.assertRaises(ProcessLookupError):os.kill(int(pid.read_text()),0)
                value=json.loads((root/'status/status.json').read_text())
                self.assertEqual(value['phase'],'stopped');self.assertIsNone(value['percent'])
            finally:
                if child.poll() is None:child.kill();child.communicate(timeout=5)

if __name__=='__main__':unittest.main()
