import base64
import hashlib
import importlib.util
import json
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
import unittest
from unittest import mock
import zipfile

SPEC = importlib.util.spec_from_file_location("diir_package", Path(__file__).parents[1] / "package.py")
package = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(package)


class PackagingTests(unittest.TestCase):
    def test_cargo_metadata_decodes_utf8_independently_of_windows_codepage(self):
        expected = {"packages": [{"name": "fixture", "authors": ["\u0141ukasz"]}]}
        payload = json.dumps(expected, ensure_ascii=False).encode("utf-8")
        with self.assertRaises(UnicodeDecodeError):
            payload.decode("cp1252")
        original_run = package.run

        def fixture_cargo(arguments, **kwargs):
            self.assertEqual(arguments[:2], ["cargo", "metadata"])
            self.assertEqual(kwargs["encoding"], "utf-8")
            return original_run([sys.executable, "-c",
                                 f"import sys; sys.stdout.buffer.write({payload!r})"], **kwargs)

        with mock.patch.object(package, "run", side_effect=fixture_cargo):
            self.assertEqual(package.metadata("x86_64-pc-windows-msvc"), expected)

    def test_cargo_metadata_reports_stderr_without_dumping_stdout(self):
        failure = subprocess.CalledProcessError(1, ["cargo", "metadata"],
                                                output="unrelated stdout payload",
                                                stderr="missing manifest fixture\n")
        with mock.patch.object(package, "run", side_effect=failure):
            with self.assertRaisesRegex(ValueError, "Cargo metadata failed: missing manifest fixture") as caught:
                package.metadata("x86_64-pc-windows-msvc")
        self.assertNotIn("unrelated stdout payload", str(caught.exception))

    def test_cargo_metadata_rejects_invalid_utf8_and_json_with_clear_errors(self):
        bad_encoding = UnicodeDecodeError("utf-8", b"\x81", 0, 1, "invalid start byte")
        with mock.patch.object(package, "run", side_effect=bad_encoding):
            with self.assertRaisesRegex(ValueError, "not valid UTF-8"):
                package.metadata("x86_64-pc-windows-msvc")
        with mock.patch.object(package, "run", return_value=subprocess.CompletedProcess(
                ["cargo", "metadata"], 0, stdout="not JSON", stderr="")):
            with self.assertRaisesRegex(ValueError, "invalid JSON"):
                package.metadata("x86_64-pc-windows-msvc")

    def test_certificate_decoder_accepts_newline_terminated_transport_encoding(self):
        certificate = bytes(range(128))
        encoded = base64.encodebytes(certificate).decode("ascii")
        self.assertTrue(encoded.endswith("\n"))
        self.assertEqual(package.decode_certificate(encoded), certificate)

    def test_certificate_decoder_accepts_wrapped_ascii_transport_whitespace(self):
        certificate = bytes(range(128))
        encoded = base64.b64encode(certificate).decode("ascii")
        wrapped = " \t\r\n".join(encoded[index:index + 12] for index in range(0, len(encoded), 12))
        self.assertEqual(package.decode_certificate("\v\f" + wrapped + "\r\n"), certificate)

    def test_certificate_decoder_rejects_empty_invalid_padding_and_non_ascii_input(self):
        for encoded in ("", " \t\r\n", "YQ==!", "YQ=", "YQ==\u00a0", "\u2603"):
            with self.subTest(encoded=repr(encoded)):
                with self.assertRaises(ValueError):
                    package.decode_certificate(encoded)

    def portable_stage(self, root, platform="windows"):
        app = package.configuration()["application"]
        if platform == "windows":
            binary = root / f"{app['binary']}.exe"
            header = bytearray(128)
            header[:2] = b"MZ"
            struct.pack_into("<I", header, 60, 64)
            header[64:68] = b"PE\0\0"
            struct.pack_into("<H", header, 68, 0x8664)
            binary.write_bytes(header)
            resources = root
            architecture = "x86_64"
        else:
            contents = root / f"{app['name']}.app/Contents"
            binary = contents / "MacOS" / app["binary"]
            binary.parent.mkdir(parents=True)
            binary.write_bytes(b"\xcf\xfa\xed\xfe" + struct.pack("<I", 0x0100000C))
            resources = contents / "Resources"
            resources.mkdir()
            architecture = "arm64"
        manifest = {
            "version": "2.3.4", "platform": platform,
            "architecture": architecture, "signing": "unsigned",
            "source_commit": hashlib.sha1(b"packaging-test-fixture").hexdigest(),
        }
        path = resources / "build-manifest.json"
        path.write_text(json.dumps(manifest), encoding="utf-8")
        return app, binary, path

    def test_application_version_comes_from_cargo_metadata(self):
        cargo = {"packages": [{"name": "diir-app", "version": "2.3.4-beta.1"}]}
        self.assertEqual(package.version_from_metadata(cargo), "2.3.4-beta.1")
        with self.assertRaises(ValueError):
            package.version_from_metadata({"packages": []})

    def test_missing_release_metadata_is_actionable(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "packaging").mkdir()
            (root / "packaging/config.toml").write_text('[application]\nname = "Test"\n')
            with self.assertRaisesRegex(ValueError, "application.package"):
                package.configuration(root)

    def test_macos_binary_must_match_requested_architecture(self):
        with tempfile.TemporaryDirectory() as directory:
            binary = Path(directory) / "binary"
            binary.write_bytes(b"\xcf\xfa\xed\xfe" + struct.pack("<I", 0x0100000C))
            package.validate_binary(binary, "macos", "arm64")
            with self.assertRaisesRegex(ValueError, "Mach-O"):
                package.validate_binary(binary, "macos", "x86_64")

    def test_windows_payload_rejects_non_pe_and_wrong_machine(self):
        with tempfile.TemporaryDirectory() as directory:
            binary = Path(directory) / "binary"
            binary.write_bytes(b"not an application")
            with self.assertRaisesRegex(ValueError, "PE"):
                package.validate_binary(binary, "windows", "x86_64")
            header = bytearray(128)
            header[:2] = b"MZ"
            struct.pack_into("<I", header, 60, 64)
            header[64:68] = b"PE\0\0"
            struct.pack_into("<H", header, 68, 0xAA64)
            binary.write_bytes(header)
            with self.assertRaisesRegex(ValueError, "x86-64"):
                package.validate_binary(binary, "windows", "x86_64")

    def test_checksum_and_manifest_describe_the_actual_artifact(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            artifact = root / "test.zip"
            artifact.write_bytes(b"artifact payload")
            manifest = {"version": "2.3.4", "signing": "unsigned"}
            package.checksums(root, artifact, manifest)
            digest, filename = (root / "test.zip.sha256").read_text().split()
            self.assertEqual(filename, artifact.name)
            self.assertEqual(digest, hashlib.sha256(artifact.read_bytes()).hexdigest())
            self.assertEqual(json.loads((root / "test.zip.json").read_text()), manifest)

    def test_ambiguous_or_missing_stage_manifest_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaises(ValueError):
                package.stage_info(root)
            for name in ("first", "second"):
                (root / name).mkdir()
                (root / name / "build-manifest.json").write_text("{}")
            with self.assertRaises(ValueError):
                package.stage_info(root)

    def test_windows_archive_contains_complete_payload_notices_and_matching_manifest(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            app = package.configuration()["application"]
            binary = root / "fixture.exe"
            _, fixture, _ = self.portable_stage(root)
            fixture.rename(binary)
            fixture_workspace = {
                "packages": [{"name": "diir-app", "version": "2.3.4", "id": "fixture-workspace"}],
                "workspace_members": ["fixture-workspace"],
            }
            source_commit = hashlib.sha1(b"packaging-test-fixture").hexdigest()
            with mock.patch.object(package, "metadata", return_value=fixture_workspace), \
                    mock.patch.object(package, "source_commit", return_value=source_commit):
                stage = package.stage("windows", "x86_64", binary, root / "output")
            artifact = package.archive(stage, root / "output", "unsigned")
            with zipfile.ZipFile(artifact) as archive:
                self.assertEqual(set(archive.namelist()), {
                    f"{app['binary']}.exe", "LICENSE", "THIRD_PARTY_NOTICES.txt", "build-manifest.json",
                })
                self.assertEqual(archive.read(f"{app['binary']}.exe"), binary.read_bytes())
                self.assertEqual(archive.read("LICENSE"), (package.ROOT / app["license_file"]).read_bytes())
                manifest = json.loads(archive.read("build-manifest.json"))
                self.assertEqual(manifest["version"], "2.3.4")
                self.assertEqual(manifest["source_commit"], source_commit)
                self.assertEqual(manifest["signing"], "unsigned")
                self.assertIn(b"does not replace dependency licenses", archive.read("THIRD_PARTY_NOTICES.txt"))
            self.assertEqual(json.loads(artifact.with_suffix(".zip.json").read_text()), manifest)
            self.assertEqual(artifact.with_suffix(".zip.sha256").read_text().split()[0],
                             hashlib.sha256(artifact.read_bytes()).hexdigest())

    def test_signed_archive_rejects_unprepared_payload_without_creating_output(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            _, _, manifest = self.portable_stage(root)
            before = manifest.read_bytes()
            output = root / "output"
            with mock.patch.object(package, "run") as native_command:
                with self.assertRaisesRegex(ValueError, "prepared before signing"):
                    package.archive(root, output, "signed")
            native_command.assert_not_called()
            self.assertEqual(manifest.read_bytes(), before)
            self.assertFalse(output.exists())

    def test_failed_signature_verification_cannot_create_a_signed_artifact(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            _, _, manifest = self.portable_stage(root)
            package.prepare_signing(root)
            before = manifest.read_bytes()
            output = root / "output"
            failure = subprocess.CalledProcessError(1, ["native-signature-verifier"])
            with mock.patch.object(package, "run", side_effect=failure):
                with self.assertRaises(subprocess.CalledProcessError):
                    package.archive(root, output, "signed")
            self.assertEqual(manifest.read_bytes(), before)
            self.assertFalse(output.exists())

    def test_mac_signed_archive_does_not_change_sealed_resources(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            _, _, manifest = self.portable_stage(root, "macos")
            package.prepare_signing(root)
            sealed_resources = manifest.read_bytes()
            calls = []

            def native_command(arguments, **kwargs):
                calls.append(arguments)
                self.assertEqual(manifest.read_bytes(), sealed_resources)
                if arguments[0] == "ditto":
                    with zipfile.ZipFile(arguments[-1], "w") as archive:
                        archive.write(manifest, manifest.relative_to(root))
                return subprocess.CompletedProcess(arguments, 0)

            with mock.patch.object(package, "run", side_effect=native_command):
                artifact = package.archive(root, root / "output", "signed")
            self.assertIn("codesign", {call[0] for call in calls})
            self.assertIn("xcrun", {call[0] for call in calls})
            self.assertEqual(manifest.read_bytes(), sealed_resources)
            self.assertEqual(json.loads(artifact.with_suffix(".zip.json").read_text())["signing"], "signed")

    def test_linux_packaging_rejects_wrong_distribution_and_version_before_tools(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = root / "build-manifest.json"
            manifest.write_text(json.dumps({"platform": "debian", "architecture": "x86_64"}))
            original_read_text = Path.read_text
            for os_release in ['ID=fedora\nVERSION_ID="45"\n', 'ID=debian\nVERSION_ID="unsupported"\n']:
                def read_text(path, *args, **kwargs):
                    if path == Path("/etc/os-release"):
                        return os_release
                    return original_read_text(path, *args, **kwargs)

                with self.subTest(os_release=os_release), \
                        mock.patch.object(Path, "read_text", autospec=True, side_effect=read_text), \
                        mock.patch.object(package, "run") as native_command:
                    with self.assertRaisesRegex(ValueError, "configured native distro baseline"):
                        package.linux_package(root, root / "output")
                    native_command.assert_not_called()

    def test_dependency_notices_preserve_explicit_and_discovered_license_texts(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            dependency = root / "dependency"
            dependency.mkdir()
            texts = {"LICENSE": "fixture license", "NOTICE": "fixture notice", "legal.txt": "fixture custom license"}
            for name, text in texts.items():
                (dependency / name).write_text(text, encoding="utf-8")
            cargo = {"workspace_members": [], "packages": [{
                "name": "fixture-dependency", "version": "1.2.3", "id": "fixture-dependency",
                "license": "MIT", "license_file": "legal.txt", "manifest_path": str(dependency / "Cargo.toml"),
            }]}
            destination = root / "THIRD_PARTY_NOTICES.txt"
            package.dependency_notices(cargo, destination)
            notices = destination.read_text()
            for text in texts.values():
                self.assertIn(text, notices)
            self.assertIn("fixture-dependency 1.2.3", notices)
            self.assertIn("License expression: MIT", notices)

    def test_missing_staged_executable_cannot_be_archived(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            _, binary, _ = self.portable_stage(root)
            binary.unlink()
            with self.assertRaisesRegex(ValueError, "Missing desktop executable"):
                package.archive(root, root / "output", "unsigned")
            self.assertFalse((root / "output").exists())


if __name__ == "__main__":
    unittest.main()
