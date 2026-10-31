"""Bounded Proton Bridge adapter, invoked over private stdin by the Rust host.
No STORE/APPEND/EXPUNGE, external resource fetching, raw-mail persistence or logging.
"""
import base64
import datetime as dt
import email.policy
import email.utils
import hashlib
import html.parser
import imaplib
import json
import re
import ssl
import sys
import unicodedata
from email.parser import BytesParser
from email.message import EmailMessage

MAX_MAIL = 2 * 1024 * 1024
RETAILERS = {
    'amazon_jp': ('amazon.co.jp',),
    'joshin': ('joshin.co.jp', 'joshinweb.jp'),
    'square_enix': ('square-enix.com',),
    'nintendo': ('nintendo.net', 'accounts.nintendo.com', 'nintendo.com'),
    'steam': ('steampowered.com',),
}
RECEIPT = re.compile(r'注文|予約|購入|出荷|発送|配達|キャンセル|返金|領収|order|purchase|receipt|refund|shipped|delivered|cancell', re.I)
LABELS = {
    'amazon_jp': r'商品名|注文商品|Product|Item',
    'joshin': r'商品名|ご注文商品|ご注文商品名|品名|商品',
    'square_enix': r'商品名|ご注文商品|商品|Product',
    'nintendo': r'ソフト名|商品名|ご購入商品|購入商品|商品|Title|Item',
    'steam': r'ゲーム名|商品名|Item|Product|Title',
}
ORDER = re.compile(r'(?:注文番号|ご注文番号|受注番号|注文受付番号|お取引番号|取引ID|Order\s*(?:number|#|ID)|Transaction\s*(?:ID|#)|Confirmation\s*(?:number|#))\s*[:：#]?\s*([A-Za-z0-9][A-Za-z0-9-]{3,99})', re.I)


def clean(text):
    value = ' '.join(unicodedata.normalize('NFKC', text).split())
    return ''.join(c for c in value if unicodedata.category(c) != 'Cc').encode('utf-8')[:1024].decode('utf-8', errors='ignore')


class TextHTML(html.parser.HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.parts = []
        self.hide = 0
        self.link = None
        self.titles = []
        self.capture = 0
        self.captured = []

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag in ('script', 'style', 'head'):
            self.hide += 1
        if tag in ('br', 'tr', 'p', 'div', 'td', 'li'):
            self.parts.append('\n')
        if tag == 'a' and re.match(r'https://store\.steampowered\.com/(?:app|sub)/\d+', attrs.get('href', '')):
            self.link = []
        if tag in ('td', 'span') and re.search(r'(?:item_name|itemname|receipt_item_name)', attrs.get('class', ''), re.I):
            self.capture = 1
            self.captured = []

    def handle_endtag(self, tag):
        if tag in ('script', 'style', 'head') and self.hide:
            self.hide -= 1
        if tag == 'a' and self.link is not None:
            self.titles.append(clean(''.join(self.link)))
            self.link = None
        if self.capture and tag in ('td', 'span'):
            self.titles.append(clean(''.join(self.captured)))
            self.capture = 0
        if tag in ('tr', 'p', 'div', 'td', 'li'):
            self.parts.append('\n')

    def handle_data(self, text):
        if not self.hide:
            self.parts.append(text)
            if self.link is not None:
                self.link.append(text)
            if self.capture:
                self.captured.append(text)


def retailer_for(message):
    sender = email.utils.parseaddr(str(message.get('From', '')))[1].lower()
    domain = sender.rpartition('@')[2]
    for retailer, domains in RETAILERS.items():
        if any(domain == allowed or domain.endswith('.' + allowed) for allowed in domains):
            return retailer
    return None


def status_for(subject):
    # Subject-only status avoids refund/cancellation help text in receipt footers.
    for pattern, status in [
        (r'返金|refund', 'refunded'), (r'キャンセル|取消|cancell', 'cancelled'),
        (r'配達完了|お届け完了|delivered', 'delivered'), (r'発送|出荷|shipped|dispatched', 'shipped'),
        (r'予約|pre.?order', 'preordered'), (r'購入|purchase|receipt|領収', 'purchased'),
        (r'注文|order', 'ordered'),
    ]:
        if re.search(pattern, subject, re.I):
            return status
    return 'unknown'


class MimeLimitError(ValueError):
    pass


def extract(raw, namespace, received_at=None):
    if len(raw) > MAX_MAIL:
        raise ValueError('mail size limit')
    try:
        return _extract(raw, namespace, received_at)
    except (ValueError, UnicodeError, LookupError, email.errors.MessageError, RecursionError) as failure:
        message = BytesParser(policy=email.policy.default).parsebytes(raw, headersonly=True)
        message._collector_raw_digest = hashlib.sha256(raw).hexdigest()
        message._collector_raw_identity_only = True
        retailer = retailer_for(message)
        if not retailer or not RECEIPT.search(str(message.get('Subject', ''))):
            return []
        return [unparsed(message, retailer, namespace, received_at, 'too_many_parts' if isinstance(failure, MimeLimitError) else 'unsupported_content')]


def _extract(raw, namespace, received_at=None):
    count = 0
    def bounded_message(*, policy):
        nonlocal count
        count += 1
        if count > 100:
            raise MimeLimitError('MIME part limit')
        return EmailMessage(policy=policy)
    policy = email.policy.default.clone(message_factory=bounded_message)
    message = BytesParser(policy=policy).parsebytes(raw)
    message._collector_raw_digest = hashlib.sha256(raw).hexdigest()
    retailer = retailer_for(message)
    subject = clean(str(message.get('Subject', '')))
    if not retailer or not RECEIPT.search(subject):
        return []
    plain, html_titles = [], []
    for part in message.walk():
        if part.get_content_disposition() == 'attachment' or part.get_content_maintype() != 'text':
            continue
        if part.get_content_type() not in ('text/plain', 'text/html'):
            continue
        data = part.get_payload(decode=True) or b''
        charset = part.get_content_charset() or 'utf-8'
        try:
            text = data.decode(charset, errors='strict')
        except (UnicodeError, LookupError):
            # Fail visibly; lossy title decoding must not silently become an identity.
            return [unparsed(message, retailer, namespace, received_at, 'unsupported_content')]
        if part.get_content_type() == 'text/html':
            parser = TextHTML()
            parser.feed(text)
            plain.append(''.join(parser.parts))
            if retailer == 'steam':
                html_titles.extend(parser.titles)
        else:
            plain.append(text)
    text = unicodedata.normalize('NFKC', '\n'.join(plain))
    order = ORDER.search(text + '\n' + subject)
    reference = order.group(1) if order else ''
    titles = html_titles
    pattern = re.compile(r'^\s*(?:[【\[])?(?:' + LABELS[retailer] + r')(?:[】\]])?\s*[:：]\s*(.*?)\s*$', re.I)
    lines = text.splitlines()
    for i, line in enumerate(lines):
        found = pattern.match(line)
        if found:
            title = found.group(1).strip()
            if not title:
                title = next((v.strip() for v in lines[i+1:i+4] if v.strip()), '')
            if title:
                titles.append(clean(title))
    if retailer == 'amazon_jp' and not titles:
        titles.extend(re.findall(r'[「“]([^」”]{2,500})[」”]', subject))
    titles = list(dict.fromkeys(clean(t) for t in titles if clean(t)))
    if not titles or len(titles) > 20:
        return [unparsed(message, retailer, namespace, received_at)]
    result = []
    for title in titles:
        platform = next((label for regex, label in [
            (r'Switch\s*2', 'Nintendo Switch 2'), (r'Switch', 'Nintendo Switch'),
            (r'PS5|PlayStation\s*5', 'PlayStation 5'), (r'PS4|PlayStation\s*4', 'PlayStation 4'),
            (r'Xbox', 'Xbox'),
        ] if re.search(regex, title, re.I)), 'PC' if retailer == 'steam' else '')
        edition = next((label for regex, label in [(r'collector|コレクター', 'Collector'), (r'deluxe|デラックス', 'Deluxe'), (r'限定|limited', 'Limited')] if re.search(regex, title, re.I)), '')
        purchase = make_purchase(message, retailer, namespace, title, reference, received_at)
        purchase.update(platform=platform, edition=edition)
        result.append(purchase)
    return result


def message_identity(message):
    identity = str(message.get('Message-ID', ''))
    if identity:
        return identity
    # Preserve supported messages' existing canonical identity. Rejected MIME
    # trees are represented by a header-only object and the exact raw digest.
    if getattr(message, '_collector_raw_identity_only', False):
        return message._collector_raw_digest
    return hashlib.sha256(message.as_bytes()).hexdigest()


def make_purchase(message, retailer, namespace, title, reference, received_at, safe_date=False):
    # A profile-scoped message+line hash survives folder duplicates and UIDVALIDITY resets.
    identity = message_identity(message)
    source = hashlib.sha256((namespace + '\0' + identity + '\0' + title).encode()).hexdigest()
    try:
        date_value = next((v for k, v in message.raw_items() if k.lower() == 'date'), None)
        when = email.utils.parsedate_to_datetime(date_value) if date_value is not None else received_at
        if when is not None and when.tzinfo is not None:
            when = when.astimezone(dt.timezone.utc)
    except (ValueError, TypeError, OverflowError):
        if not safe_date:
            return unparsed(message, retailer, namespace, received_at, 'malformed_date')
        when = received_at
    if when is None or when.tzinfo is None:
        when = received_at or dt.datetime(1970, 1, 1, tzinfo=dt.timezone.utc)
    return dict(id=source, retailer=retailer, order_reference=reference, title=title, platform='', edition='',
                format='digital' if retailer in ('steam', 'nintendo') else 'unknown',
                status=status_for(clean(str(message.get('Subject', '')))), observed_at=when.astimezone(dt.timezone.utc).isoformat(),
                intentional_duplicate=False, source_keys=[source])


def unparsed(message, retailer, namespace, received_at, issue='unrecognized_layout'):
    reference = ORDER.search(clean(str(message.get('Subject', ''))))
    value = make_purchase(message, retailer, namespace, 'Unparsed receipt — review original email', reference.group(1) if reference else '', received_at, safe_date=True)
    value['status'] = 'unknown'
    value['_issue'] = issue
    return value


def imap_folder(name):
    """RFC 3501 modified UTF-7; IMAP4 mailbox commands otherwise accept ASCII."""
    output, pending = [], []
    def flush():
        if pending:
            encoded = base64.b64encode(''.join(pending).encode('utf-16-be')).decode('ascii')
            output.append('&' + encoded.rstrip('=').replace('/', ',') + '-')
            pending.clear()
    for char in name:
        if 0x20 <= ord(char) <= 0x7e:
            flush()
            output.append('&-' if char == '&' else char)
        else:
            pending.append(char)
    flush()
    return ''.join(output)


class MailError(ValueError):
    def __init__(self, kind, folder=None):
        self.kind, self.folder = kind, folder
        super().__init__('Mail operation failed')


def checked(result, kind='invalid_response', folder=None):
    status, data = result
    if status != 'OK':
        raise MailError(kind, folder)
    return data


def bounded_header(header, field, maximum):
    original = next((v for k, v in header.raw_items() if k.lower() == 'date'), '') if field == 'Date' else str(header.get(field, ''))
    value = ' '.join(original.split())
    value = ''.join(c for c in value if not unicodedata.category(c).startswith('C'))
    return value.encode('utf-8')[:maximum].decode('utf-8', errors='ignore')


def folder_progress(folder, validity, last, end):
    return dict(folder=folder, uid_validity=validity, scanned_through=last,
                available_through=end, caught_up=last==end,
                checked_at=dt.datetime.now(dt.timezone.utc).isoformat())


def fetch_batch(profile, cursor, probe=False, auth_only=False):
    if any(not isinstance(profile.get(field), str) or any(ord(c) < 32 or ord(c) == 127 for c in profile[field]) for field in ('username', 'password')):
        raise ValueError('invalid local credentials')
    if profile.get('host', '127.0.0.1') != '127.0.0.1':
        raise ValueError('Bridge must use loopback')
    state = json.loads(cursor) if cursor else {}
    folders = profile.get('folders', ['INBOX'])
    if not 1 <= len(folders) <= 16:
        raise ValueError('folder limit')
    client = imaplib.IMAP4('127.0.0.1', profile.get('port', 1143), timeout=15)
    try:
        # Bridge uses a self-signed certificate. Pin its exact DER digest before LOGIN.
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
        context.check_hostname = False
        context.verify_mode = ssl.CERT_NONE
        checked(client.starttls(context))
        actual = hashlib.sha256(client.sock.getpeercert(binary_form=True)).hexdigest()
        import hmac
        if not hmac.compare_digest(actual, profile['certificate_sha256'].lower()):
            raise MailError('certificate')
        try:
            checked(client.login(profile['username'], profile['password']), 'authentication')
        except imaplib.IMAP4.error:
            raise MailError('authentication') from None
        if auth_only:
            return dict(cursor='', purchases=[])
        purchases, details, progress = [], [], []
        for folder in folders:
            if any(c in folder for c in '\r\n\x00') or len(folder) > 256:
                raise ValueError('invalid folder')
            quoted = '"' + imap_folder(folder).replace('\\', '\\\\').replace('"', '\\"') + '"'
            checked(client.select(quoted, readonly=True), 'folder', folder)  # EXAMINE, never SELECT writable.
            status = checked(client.status(quoted, '(UIDVALIDITY UIDNEXT)'), 'folder', folder)[0]
            validity = int(re.search(rb'UIDVALIDITY (\d+)', status).group(1))
            uidnext = int(re.search(rb'UIDNEXT (\d+)', status).group(1))
            if probe:
                continue
            previous = state.get(folder, {})
            last = previous.get('uid', 0) if previous.get('validity') == validity else 0
            end = min(last + 100, uidnext - 1)
            if end > last:
                uids = checked(client.uid('search', None, 'UID', f'{last+1}:{end}'))[0].split()
                for uid in uids:
                    data = checked(client.uid('fetch', uid, '(RFC822.SIZE INTERNALDATE BODY.PEEK[HEADER.FIELDS (FROM SUBJECT MESSAGE-ID DATE)])'))
                    parts = [x for x in data if isinstance(x, tuple)]
                    if not parts:
                        raise ValueError('missing header')
                    header = BytesParser(policy=email.policy.default).parsebytes(parts[0][1])
                    internal = re.search(rb'INTERNALDATE "([^"]+)"', parts[0][0])
                    received_at = None
                    if internal:
                        received_at = email.utils.parsedate_to_datetime(internal.group(1).decode('ascii').replace('-', ' '))
                    if retailer_for(header) and RECEIPT.search(str(header.get('Subject', ''))):
                        size = int(re.search(rb'RFC822.SIZE (\d+)', parts[0][0]).group(1))
                        if size > MAX_MAIL:
                            extracted = [unparsed(header, retailer_for(header), profile['id'], received_at, 'too_large')]
                        else:
                            full = checked(client.uid('fetch', uid, f'(BODY.PEEK[]<0.{MAX_MAIL+1}>)'))
                            literal = [x[1] for x in full if isinstance(x, tuple)]
                            if len(literal) != 1 or len(literal[0]) != size:
                                raise ValueError('incomplete message')
                            extracted = extract(literal[0], profile['id'], received_at)
                        if len(purchases) + len(extracted) > 100:
                            state[folder] = dict(validity=validity, uid=int(uid)-1)
                            progress.append(folder_progress(folder, validity, int(uid)-1, uidnext-1))
                            return dict(cursor=json.dumps(state), purchases=purchases, details=details, progress=progress)
                        for purchase in extracted:
                            issue = purchase.pop('_issue', None)
                            details.append(dict(key=purchase['id'], issue=issue, source=dict(
                                folder=folder, uid_validity=validity, uid=int(uid),
                                message_id=bounded_header(header, 'Message-ID', 512),
                                subject=bounded_header(header, 'Subject', 512),
                                date=bounded_header(header, 'Date', 128))))
                        purchases.extend(extracted)
                state[folder] = dict(validity=validity, uid=end)
            else:
                state[folder] = dict(validity=validity, uid=last)
            progress.append(folder_progress(folder, validity, min(state[folder]['uid'], uidnext-1), uidnext-1))
            if len(purchases) >= 80:
                break
        return dict(cursor=json.dumps(state), purchases=purchases, details=details, progress=progress)
    finally:
        try:
            client.logout()
        except Exception:
            pass


if __name__ == '__main__':
    try:
        request = json.load(sys.stdin)
        result = fetch_batch(request['profile'], request['cursor'], request.get('operation') == 'probe', request.get('operation') == 'authenticate')
        sys.stdout.write(json.dumps(result, ensure_ascii=False))
    except Exception as failure:
        # Only fixed typed categories and the submitted folder cross this boundary.
        kind = failure.kind if isinstance(failure, MailError) else ('transport' if isinstance(failure, (OSError, imaplib.IMAP4.abort)) else 'invalid_response')
        folder = failure.folder if isinstance(failure, MailError) else None
        sys.stdout.write(json.dumps(dict(cursor='', purchases=[], failure=dict(kind=kind, folder=folder))))
        # No exception repr: libraries can include credentials, headers or mail bodies.
        sys.stderr.write('Mail adapter failed; inspect local configuration.\n')
        sys.exit(0)
