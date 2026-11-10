"""Bounded Proton Bridge adapter, invoked over private stdin by the Rust host.
No STORE/APPEND/EXPUNGE, external resource fetching, raw-mail persistence or logging.
"""
import base64
from contextlib import contextmanager
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
from urllib.parse import parse_qs, urlsplit

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
AMAZON_PHASE = re.compile(r'^(ordered|shipped|out for delivery|delivered)\b', re.I)
AMAZON_ORDER = re.compile(r'(?:注文番号|\bOrder\s*(?:number\b|#))\s*:?\s*([0-9]{3}-[0-9]{7}-[0-9]{7})(?![\w-])', re.I)
BIDI_CONTROLS = str.maketrans('', '', '\u061c\u200e\u200f\u202a\u202b\u202c\u202d\u202e\u2066\u2067\u2068\u2069')
UNPARSED_TITLE = 'Unparsed receipt — review original email'


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
        if tag == 'a' and re.match(r'https://store\.steampowered\.com/(?:app|sub)/\d+', attrs.get('href') or ''):
            self.link = []
        if tag in ('td', 'span') and re.search(r'(?:item_name|itemname|receipt_item_name)', attrs.get('class') or '', re.I):
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


def amazon_product_link(href):
    """Identify item links in the order card using only their inline markup."""
    try:
        redirect = urlsplit(href)
        if (redirect.scheme, redirect.netloc, redirect.path) != ('https', 'www.amazon.co.jp', '/gp/r.html'):
            return None
        query = parse_qs(redirect.query, max_num_fields=32)
        marker, = query.get('ref_', [])
        target, = query.get('U', [])
        match = re.search(r'_(i|t)_fed_asin_title$', marker)
        if match:
            product = urlsplit(target)
            asin = re.fullmatch(r'/dp/([A-Z0-9]{10})/?', product.path)
            if (product.scheme, product.netloc) == ('https', 'www.amazon.co.jp') and asin:
                return asin[1], match[1]
    except ValueError:
        pass
    return None


class AmazonHTML(TextHTML):
    def __init__(self):
        super().__init__()
        self.products = {}
        self.item = None
        self.item_text = []

    def handle_starttag(self, tag, attrs):
        super().handle_starttag(tag, attrs)
        if self.hide:
            return
        if tag == 'a':
            self.item = amazon_product_link(dict(attrs).get('href') or '')
            self.item_text = []
            if self.item:
                self.products.setdefault(self.item[0], {})
        elif tag == 'img' and self.item and self.item[1] == 'i':
            self.item_text.append(dict(attrs).get('alt') or '')

    def handle_endtag(self, tag):
        super().handle_endtag(tag)
        if tag == 'a' and self.item:
            asin, kind = self.item
            if title := clean(''.join(self.item_text).translate(BIDI_CONTROLS)):
                self.products[asin].setdefault(kind, set()).add(title)
            self.item = None

    def handle_data(self, text):
        super().handle_data(text)
        if not self.hide and self.item and self.item[1] == 't':
            self.item_text.append(text)

    def order_items(self):
        text = unicodedata.normalize('NFKC', ''.join(self.parts)).translate(BIDI_CONTROLS)
        references = set(AMAZON_ORDER.findall(text))
        if len(references) != 1 or len(self.products) > 20:
            return None
        items = []
        for asin, names in sorted(self.products.items()):
            link_titles = names.get('t', set())
            alt_titles = names.get('i', set())
            if len(link_titles) > 1:
                items.append((asin, None))
                continue
            title = next(iter(link_titles), '')
            if not title or title.endswith('...'):
                prefix = title.removesuffix('...').rstrip().casefold()
                alt = next(iter(alt_titles), '') if len(alt_titles) == 1 else ''
                title = alt if alt and not alt.endswith('...') and alt.casefold().startswith(prefix) else None
            items.append((asin, title))
        reference, = references
        return reference, tuple(items)


def receipt_parts(message):
    if message.get_content_disposition() == 'attachment':
        return
    if message.get_content_maintype() == 'multipart':
        for part in message.iter_parts():
            yield from receipt_parts(part)
    elif message.get_content_type() in ('text/plain', 'text/html'):
        yield message


def retailer_for(message):
    sender = email.utils.parseaddr(str(message.get('From', '')))[1].lower()
    domain = sender.rpartition('@')[2]
    for retailer, domains in RETAILERS.items():
        if any(domain == allowed or domain.endswith('.' + allowed) for allowed in domains):
            return retailer
    return None


def receipt_subject(retailer, subject):
    return retailer and (RECEIPT.search(subject)
                         or retailer == 'amazon_jp' and AMAZON_PHASE.match(clean(subject)))


def status_for(retailer, subject):
    # Subject-only status avoids refund/cancellation help text in receipt footers.
    if retailer == 'amazon_jp' and (phase := AMAZON_PHASE.match(subject)):
        return phase[1].lower().replace(' ', '_')
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
        if not receipt_subject(retailer, str(message.get('Subject', ''))):
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
    if not receipt_subject(retailer, subject):
        return []
    plain, html_titles, amazon_cards = [], [], []
    for part in receipt_parts(message):
        data = part.get_payload(decode=True) or b''
        charset = part.get_content_charset() or 'utf-8'
        try:
            text = data.decode(charset, errors='strict')
        except (UnicodeError, LookupError):
            # Fail visibly; lossy title decoding must not silently become an identity.
            return [unparsed(message, retailer, namespace, received_at, 'unsupported_content')]
        if part.get_content_type() == 'text/html':
            parser = AmazonHTML() if retailer == 'amazon_jp' else TextHTML()
            parser.feed(text)
            parser.close()
            plain.append(''.join(parser.parts))
            if retailer == 'steam':
                html_titles.extend(parser.titles)
            if isinstance(parser, AmazonHTML) and parser.products:
                amazon_cards.append(parser.order_items())
        else:
            plain.append(text)
    if amazon_cards and AMAZON_PHASE.match(subject):
        card = amazon_cards[0]
        if card is None or any(other != card for other in amazon_cards[1:]):
            return [unparsed(message, retailer, namespace, received_at)]
        reference, items = card
        purchases = []
        seen_titles = set()
        for asin, title in items:
            # Keep the existing title-based source key for the first ASIN with a
            # resolved title so previously reviewed or dismissed sources still match.
            item_key = asin if not title or title in seen_titles else None
            purchase = make_purchase(message, retailer, namespace, title or UNPARSED_TITLE,
                                     reference, received_at, item_key=item_key)
            if '_issue' in purchase:
                return [purchase]
            purchase['format'] = 'physical'
            if title:
                seen_titles.add(title)
                purchase['_amazon_order_card'] = True
            else:
                purchase['_issue'] = 'incomplete_item_title'
            purchases.append(purchase)
        return purchases
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


def make_purchase(message, retailer, namespace, title, reference, received_at, safe_date=False, item_key=None):
    # A profile-scoped message+line hash survives folder duplicates and UIDVALIDITY resets.
    identity = message_identity(message)
    source_input = namespace + '\0' + identity + '\0' + title
    if item_key is not None:
        source_input += '\0' + item_key
    source = hashlib.sha256(source_input.encode()).hexdigest()
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
                status=status_for(retailer, clean(str(message.get('Subject', '')))), observed_at=when.astimezone(dt.timezone.utc).isoformat(),
                intentional_duplicate=False, source_keys=[source])


def unparsed(message, retailer, namespace, received_at, issue='unrecognized_layout'):
    reference = ORDER.search(clean(str(message.get('Subject', ''))))
    value = make_purchase(message, retailer, namespace, UNPARSED_TITLE, reference.group(1) if reference else '', received_at, safe_date=True)
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


def decode_imap_folder(raw):
    encoded = raw.decode('ascii')

    def decode_shift(match):
        value = match[1].replace(',', '/')
        if not value:
            return '&'
        return base64.b64decode(value + '=' * (-len(value) % 4), validate=True).decode('utf-16-be')

    name = re.sub(r'&([A-Za-z0-9+,]*)-', decode_shift, encoded)
    if imap_folder(name) != encoded:
        raise ValueError('invalid modified UTF-7 mailbox name')
    return name


LIST_ROW = re.compile(rb'\(([^()]*)\) (?:NIL|"(?:\\["\\]|[^"\\])") (.+)', re.I)
QUOTED_MAILBOX = re.compile(rb'"((?:\\["\\]|[^"\\])*)"')
ATOM_MAILBOX = re.compile(rb'[^(){%*"\\\x00-\x20\x7f-\xff]+')
LITERAL_MAILBOX = re.compile(rb'\{([0-9]{1,4})\}')


def list_folders(client):
    rows = checked(client.list('""', '*'))
    if rows == [None]:
        return []
    folders = set()
    replies = iter(rows)
    for row in replies:
        prefix, literal = row if isinstance(row, tuple) else (row, None)
        parsed = LIST_ROW.fullmatch(prefix)
        if not parsed:
            raise MailError('invalid_response')
        flags, token = parsed.groups()
        if literal is not None:
            size = LITERAL_MAILBOX.fullmatch(token)
            if size is None or int(size[1]) != len(literal) or next(replies, None) != b'':
                raise MailError('invalid_response')
            raw = literal
        elif quoted := QUOTED_MAILBOX.fullmatch(token):
            raw = re.sub(rb'\\(["\\])', rb'\1', quoted[1])
        elif ATOM_MAILBOX.fullmatch(token):
            raw = token
        else:
            raise MailError('invalid_response')
        if {b'\\noselect', b'\\nonexistent'}.intersection(flags.lower().split()):
            continue
        name = decode_imap_folder(raw)
        # Use the same name limits as the application folder setting.
        if not name.strip() or len(name.encode('utf-8')) > 256 or any(
            unicodedata.category(c) == 'Cc' for c in name
        ):
            continue
        folders.add('INBOX' if name.upper() == 'INBOX' and name.isascii() else name)
        if len(folders) > 4096:
            raise MailError('invalid_response')
    return sorted(folders, key=lambda name: (name != 'INBOX', name.casefold(), name))


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


class DiscoveryIMAP(imaplib.IMAP4):
    """Bound LIST response accumulation, including literals, before imaplib stores it."""
    remaining = 1024 * 1024

    def readline(self):
        line = super().readline()
        self.remaining -= len(line)
        if self.remaining < 0:
            raise MailError('invalid_response')
        return line

    def read(self, size):
        if size > 1024 or size > self.remaining:
            raise MailError('invalid_response')
        self.remaining -= size
        return super().read(size)


@contextmanager
def bridge_session(profile, discover=False):
    if any(not isinstance(profile.get(field), str) or any(ord(c) < 32 or ord(c) == 127 for c in profile[field]) for field in ('username', 'password')):
        raise ValueError('invalid local credentials')
    if profile.get('host', '127.0.0.1') != '127.0.0.1':
        raise ValueError('Bridge must use loopback')
    connect = DiscoveryIMAP if discover else imaplib.IMAP4
    client = connect('127.0.0.1', profile.get('port', 1143), timeout=15)
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
        yield client
    finally:
        try:
            client.logout()
        except Exception:
            pass


def fetch_batch(profile, cursor, probe=False, auth_only=False):
    state = json.loads(cursor) if cursor else {}
    folders = profile.get('folders', ['INBOX'])
    if not 1 <= len(folders) <= 16:
        raise ValueError('folder limit')
    with bridge_session(profile) as client:
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
                    retailer = retailer_for(header)
                    if receipt_subject(retailer, str(header.get('Subject', ''))):
                        size = int(re.search(rb'RFC822.SIZE (\d+)', parts[0][0]).group(1))
                        if size > MAX_MAIL:
                            extracted = [unparsed(header, retailer, profile['id'], received_at, 'too_large')]
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
                            amazon_order_card = purchase.pop('_amazon_order_card', False)
                            details.append(dict(key=purchase['id'], issue=issue, amazon_order_card=amazon_order_card, source=dict(
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


if __name__ == '__main__':
    try:
        request = json.load(sys.stdin)
        operation = request.get('operation')
        if operation == 'list':
            with bridge_session(request['profile'], discover=True) as client:
                result = dict(folders=list_folders(client))
        else:
            result = fetch_batch(request['profile'], request['cursor'], operation == 'probe', operation == 'authenticate')
        sys.stdout.write(json.dumps(result, ensure_ascii=False))
    except Exception as failure:
        # Only fixed typed categories and the submitted folder cross this boundary.
        kind = failure.kind if isinstance(failure, MailError) else ('transport' if isinstance(failure, (OSError, imaplib.IMAP4.abort)) else 'invalid_response')
        folder = failure.folder if isinstance(failure, MailError) else None
        sys.stdout.write(json.dumps(dict(cursor='', purchases=[], failure=dict(kind=kind, folder=folder))))
        # No exception repr: libraries can include credentials, headers or mail bodies.
        sys.stderr.write('Mail adapter failed; inspect local configuration.\n')
        sys.exit(0)
