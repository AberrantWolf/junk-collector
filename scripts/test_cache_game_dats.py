"""Offline regressions: never contact catalog providers."""
import io
import json
import tempfile
import unittest
import zipfile
from pathlib import Path
from unittest.mock import patch
import cache_game_dats as cache


class NoNetwork:
    def open(self, *_args, **_kwargs):
        raise AssertionError("network access must not occur")


class CacheTests(unittest.TestCase):
    def test_paused_provider_never_contacts_network(self):
        with tempfile.TemporaryDirectory() as root:
            with self.assertRaises(cache.ProviderRefusal):
                cache.cache_catalog(NoNetwork(), Path(root), cache.CATALOGS['no-intro-n64'],
                                    queue_attempts=0, queue_delay=180)

    def test_form_rejects_unadvertised_values(self):
        form = '<form name="main_form"><input name="format" value="2" checked><input type="submit" name="prepare_token" value="Prepare"></form>'
        with self.assertRaisesRegex(RuntimeError, 'unadvertised'):
            cache.datomatic_form(cache.CATALOGS['no-intro-nes'], form)
        form = form.replace('value="2"', 'value="1"')
        self.assertEqual(cache.datomatic_form(cache.CATALOGS['no-intro-nes'], form),
                         {'format': '1', 'prepare_token': 'Prepare'})

    def test_refusal_is_not_a_missing_download(self):
        with self.assertRaises(cache.ProviderRefusal):
            cache.read_html(io.BytesIO(b'Unknown filter. Error id: 1260'))

    def test_download_refusal_aborts_before_writing_an_archive(self):
        class Response(io.BytesIO):
            headers = {'Content-Type': 'text/html'}
        with tempfile.TemporaryDirectory() as root:
            destination = Path(root)/'source.zip'
            with self.assertRaises(cache.ProviderRefusal):
                cache.save_response(Response(b'Unknown filter. Error id: 1260'), destination)
            self.assertFalse(destination.exists())

    def test_zip_expansion_is_bounded_and_preserves_existing_output(self):
        with tempfile.TemporaryDirectory() as root:
            archive, output = Path(root)/'source.zip', Path(root)/'source.dat'
            with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED) as z:
                z.writestr('big.dat', b'x'*4096)
            output.write_bytes(b'previous')
            with patch.object(cache, 'MAX_DAT_BYTES', 1024):
                with self.assertRaisesRegex(RuntimeError, 'expanded DAT'):
                    cache.extract_single_dat(archive, output)
            self.assertEqual(output.read_bytes(), b'previous')
            self.assertFalse(output.with_suffix('.dat.part').exists())

    def test_failed_download_is_not_published(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            def broken(_client, _catalog, archive):
                archive.write_bytes(b'not a ZIP')
                return 'https://example.invalid'
            with patch.object(cache, 'fetch_redump', broken):
                with self.assertRaises(zipfile.BadZipFile):
                    cache.cache_catalog(NoNetwork(), root, cache.CATALOGS['redump-psx'],
                                        queue_attempts=0, queue_delay=180)
            self.assertEqual(list((root/'redump').iterdir()), [])

    def test_complete_cache_is_published_and_reused_without_network(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            def downloaded(_client, _catalog, archive):
                with zipfile.ZipFile(archive, 'w') as z:
                    z.writestr('catalog.dat', '<datafile><header><name>Sony - PlayStation</name><version>fixture-1</version></header><game name="Game"><rom name="Game.bin" size="2352" crc="11111111"/></game></datafile>')
                return 'https://example.invalid'
            def run():
                cache.cache_catalog(NoNetwork(), root, cache.CATALOGS['redump-psx'],
                                    queue_attempts=0, queue_delay=180)
            with patch.object(cache, 'fetch_redump', downloaded):
                run()
            run()
            directory = root/'redump'/'redump-psx'
            manifest = json.loads((directory/'manifest.json').read_text())
            self.assertEqual(manifest['document_version'], 'fixture-1')
            self.assertEqual(list((root/'redump').iterdir()), [directory])

    def test_download_system_is_checked_before_publication(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            def downloaded(_client, _catalog, archive):
                with zipfile.ZipFile(archive, 'w') as z:
                    z.writestr('catalog.dat', '<datafile><header><name>Wrong system</name><version>1</version></header></datafile>')
                return 'https://example.invalid'
            with patch.object(cache, 'fetch_redump', downloaded):
                with self.assertRaisesRegex(RuntimeError, 'unexpected system'):
                    cache.cache_catalog(NoNetwork(), root, cache.CATALOGS['redump-psx'],
                                        queue_attempts=0, queue_delay=180)
            self.assertEqual(list((root/'redump').iterdir()), [])

    def test_cache_reuse_checks_bytes_and_refresh_cannot_overwrite(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            directory = root/'no-intro'/'no-intro-nes'
            directory.mkdir(parents=True)
            record = {'catalog_key': 'no-intro-nes'}
            for key, filename in [('archive', 'source.zip'), ('source_dat', 'source.dat'), ('dat', 'catalog.dat')]:
                path = directory/filename
                path.write_bytes(b'fixture')
                record[key] = {'path': filename, 'size': path.stat().st_size, 'sha256': cache.sha256(path)}
            (directory/'manifest.json').write_text(json.dumps(record))
            def run():
                cache.cache_catalog(NoNetwork(), root, cache.CATALOGS['no-intro-nes'],
                                    queue_attempts=0, queue_delay=180)
            run()
            (directory/'catalog.dat').write_bytes(b'changed')
            with self.assertRaisesRegex(RuntimeError, 'incomplete or changed'):
                run()


if __name__ == '__main__':
    unittest.main()
