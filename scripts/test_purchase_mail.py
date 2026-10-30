#!/usr/bin/env python3
"""Synthetic, offline receipt/IMAP tests. No account, mailbox or retailer requests."""
import importlib.util
import json
from pathlib import Path
import unittest
from email.message import EmailMessage
from unittest.mock import patch
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
    def test_pin_mismatch_never_sends_credentials(self):
        FakeIMAP.commands=[]
        with patch.object(mail.imaplib,'IMAP4',FakeIMAP):
            with self.assertRaises(ValueError):mail.fetch_batch(dict(id='p',username='u',password='p',certificate_sha256='0'*64),'')
        self.assertNotIn('LOGIN',FakeIMAP.commands)
if __name__=='__main__':unittest.main()
