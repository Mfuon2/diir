#!/usr/bin/env python3
"""Stage and validate native artifacts; invoke distro tools only on that distro."""
from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import plistlib
import re
import shutil
import struct
import subprocess
import sys
import tempfile
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]
TARGETS = {
    ("macos", "arm64"): "aarch64-apple-darwin",
    ("macos", "x86_64"): "x86_64-apple-darwin",
    ("windows", "x86_64"): "x86_64-pc-windows-msvc",
    ("debian", "x86_64"): "x86_64-unknown-linux-gnu",
    ("fedora", "x86_64"): "x86_64-unknown-linux-gnu",
}


def run(arguments: list[str], **kwargs) -> subprocess.CompletedProcess:
    return subprocess.run(arguments, check=True, text=True, **kwargs)


def decode_certificate(value: str) -> bytes:
    """Accept Base64 transport whitespace without accepting other invalid data."""
    encoded = value.encode("ascii").translate(None, b" \t\r\n\v\f")
    if not encoded:
        raise ValueError("Certificate encoding is empty")
    return base64.b64decode(encoded, validate=True)


def configuration(root: Path = ROOT) -> dict:
    with (root / "packaging/config.toml").open("rb") as source:
        config = tomllib.load(source)
    required = ("name", "package", "binary", "bundle_identifier", "maintainer", "homepage",
                "summary", "description", "license", "license_file", "revision")
    app = config.get("application", {})
    for key in required:
        if not isinstance(app.get(key), str) or not app[key].strip():
            raise ValueError(f"Missing packaging application.{key}")
        if "\n" in app[key] or "\r" in app[key]:
            raise ValueError(f"Packaging application.{key} must be a single line")
    for key in ("binary", "package"):
        if not re.fullmatch(r"[a-z][a-z0-9-]+", app[key]):
            raise ValueError(f"Invalid application.{key}")
    if not re.fullmatch(r"[A-Za-z0-9]+(?:[.-][A-Za-z0-9]+)+", app["bundle_identifier"]):
        raise ValueError("Invalid macOS bundle identifier")
    if not (root / app["license_file"]).is_file():
        raise ValueError("Project license text is missing")
    return config


def metadata(target: str) -> dict:
    try:
        result = run(["cargo", "metadata", "--locked", "--format-version", "1",
                      "--filter-platform", target], cwd=ROOT, capture_output=True,
                     encoding="utf-8")
    except UnicodeDecodeError as error:
        raise ValueError("Cargo metadata output is not valid UTF-8") from error
    except subprocess.CalledProcessError as error:
        detail = error.stderr.strip() if error.stderr else f"exit code {error.returncode}"
        raise ValueError(f"Cargo metadata failed: {detail}") from error
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise ValueError("Cargo metadata returned invalid JSON") from error


def version_from_metadata(cargo: dict) -> str:
    package = next((item for item in cargo["packages"] if item["name"] == "diir-app"), None)
    if package is None or not re.fullmatch(r"\d+\.\d+\.\d+(?:-[A-Za-z0-9.-]+)?", package["version"]):
        raise ValueError("Cannot determine a supported Diir version from Cargo metadata")
    return package["version"]


def validate_binary(path: Path, platform: str, architecture: str) -> None:
    if not path.is_file():
        raise ValueError(f"Missing desktop executable: {path}")
    with path.open("rb") as source:
        header = source.read(4096)
    if platform == "macos":
        expected = 0x0100000C if architecture == "arm64" else 0x01000007
        if len(header) < 8 or header[:4] != b"\xcf\xfa\xed\xfe" or struct.unpack_from("<I", header, 4)[0] != expected:
            raise ValueError("Expected a native 64-bit Mach-O of the requested architecture")
    elif platform == "windows":
        if len(header) < 64 or header[:2] != b"MZ":
            raise ValueError("Expected a Windows PE executable")
        offset = struct.unpack_from("<I", header, 60)[0]
        with path.open("rb") as source:
            source.seek(offset)
            pe = source.read(6)
        if pe[:4] != b"PE\0\0" or len(pe) < 6 or struct.unpack_from("<H", pe, 4)[0] != 0x8664:
            raise ValueError("Expected an x86-64 Windows PE executable")
    elif len(header) < 20 or header[:6] != b"\x7fELF\x02\x01" or struct.unpack_from("<H", header, 18)[0] != 62:
        raise ValueError("Expected an x86-64 Linux ELF executable")


def dependency_notices(cargo: dict, destination: Path) -> None:
    lines = ["Third-party dependency metadata and bundled license texts",
             "Diir's Apache-2.0 license does not replace dependency licenses.", ""]
    members = set(cargo["workspace_members"])
    for package in sorted(cargo["packages"], key=lambda item: (item["name"], item["version"])):
        if package["id"] in members:
            continue
        lines.extend([f"{package['name']} {package['version']}",
                      f"License expression: {package.get('license') or 'Not declared in Cargo metadata'}"])
        directory = Path(package["manifest_path"]).parent
        files = set()
        if package.get("license_file"):
            files.add(directory / package["license_file"])
        for item in directory.iterdir():
            if item.is_file() and item.name.upper().startswith(("LICENSE", "COPYING", "COPYRIGHT", "NOTICE")):
                files.add(item)
        for item in sorted(files):
            if item.is_file():
                lines.extend([f"--- {item.name} ---", item.read_text(encoding="utf-8", errors="replace")])
        lines.append("")
    destination.write_text("\n".join(lines), encoding="utf-8")


def source_commit() -> str:
    value = os.environ.get("GITHUB_SHA") or run(["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True).stdout.strip()
    if not re.fullmatch(r"[0-9a-f]{40,64}", value):
        raise ValueError("Invalid source commit identifier")
    return value


def stage(platform: str, architecture: str, binary: Path, output: Path) -> Path:
    target = TARGETS.get((platform, architecture))
    if target is None:
        raise ValueError("Unsupported platform/architecture pair")
    config = configuration()
    app = config["application"]
    validate_binary(binary, platform, architecture)
    if platform == "macos":
        libraries = run(["otool", "-L", str(binary)], capture_output=True).stdout.splitlines()[1:]
        for library in libraries:
            name = library.strip().split(" (", 1)[0]
            if not name.startswith(("/System/Library/", "/usr/lib/")):
                raise ValueError(f"Unbundled non-system macOS dependency: {name}")
    cargo = metadata(target)
    version = version_from_metadata(cargo)
    destination = output.resolve() / "stage" / f"{platform}-{architecture}"
    if destination.exists():
        raise ValueError(f"Stage already exists; choose a fresh output directory: {destination}")
    destination.mkdir(parents=True)
    if platform == "macos":
        contents = destination / f"{app['name']}.app" / "Contents"
        executable = contents / "MacOS" / app["binary"]
        notices = contents / "Resources"
        executable.parent.mkdir(parents=True)
        notices.mkdir()
        info = {
            "CFBundleIdentifier": app["bundle_identifier"], "CFBundleName": app["name"],
            "CFBundleDisplayName": app["name"], "CFBundleExecutable": app["binary"],
            "CFBundlePackageType": "APPL", "CFBundleShortVersionString": version,
            "CFBundleVersion": version.split("-")[0], "NSHighResolutionCapable": True,
            "LSMinimumSystemVersion": config["targets"]["macos_minimum"],
        }
        (contents / "Info.plist").write_bytes(plistlib.dumps(info))
    elif platform == "windows":
        executable = destination / f"{app['binary']}.exe"
        notices = destination
    else:
        executable = destination / "usr/bin" / app["binary"]
        notices = destination / "usr/share/doc" / app["package"]
        executable.parent.mkdir(parents=True)
        notices.mkdir(parents=True)
        applications = destination / "usr/share/applications"
        applications.mkdir(parents=True)
        shutil.copy2(ROOT / "packaging/linux/diir.desktop", applications / f"{app['package']}.desktop")
        licenses = destination / "usr/share/licenses" / app["package"]
        licenses.mkdir(parents=True)
        shutil.copy2(ROOT / app["license_file"], licenses / "LICENSE")
    shutil.copy2(binary, executable)
    executable.chmod(0o755)
    if platform in ("macos", "windows"):
        shutil.copy2(ROOT / app["license_file"], notices / "LICENSE")
    dependency_notices(cargo, notices / "THIRD_PARTY_NOTICES.txt")
    manifest = {"application": app["name"], "version": version, "platform": platform,
                "architecture": architecture, "target": target, "source_commit": source_commit(),
                "signing": "unsigned", "license": app["license"],
                "distro_baseline": config["targets"].get(platform)}
    (notices / "build-manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    return destination


def stage_info(destination: Path) -> tuple[dict, Path]:
    files = list(destination.rglob("build-manifest.json"))
    if len(files) != 1:
        raise ValueError("Stage must contain exactly one build manifest")
    return json.loads(files[0].read_text(encoding="utf-8")), files[0]


def checksums(output: Path, artifact: Path, manifest: dict) -> None:
    digest = hashlib.sha256()
    with artifact.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    (output / f"{artifact.name}.sha256").write_text(f"{digest.hexdigest()}  {artifact.name}\n", encoding="utf-8")
    (output / f"{artifact.name}.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


def archive(destination: Path, output: Path, signing: str) -> Path:
    config = configuration()
    app = config["application"]
    manifest, manifest_path = stage_info(destination)
    platform, architecture = manifest["platform"], manifest["architecture"]
    if platform not in ("macos", "windows") or signing not in ("unsigned", "signed"):
        raise ValueError("Archive requires a macOS/Windows stage and a valid signing mode")
    executable = destination / (f"{app['name']}.app/Contents/MacOS/{app['binary']}" if platform == "macos" else f"{app['binary']}.exe")
    validate_binary(executable, platform, architecture)
    manifest["signing"] = signing
    # Set metadata before macOS signing, since the resources are covered by codesign.
    if signing == "signed":
        if json.loads(manifest_path.read_text())["signing"] != "signed":
            raise ValueError("Signed archive requires metadata prepared before signing")
        if platform == "macos":
            bundle = destination / f"{app['name']}.app"
            run(["codesign", "--verify", "--deep", "--strict", str(bundle)])
            run(["xcrun", "stapler", "validate", str(bundle)])
        else:
            literal = str(executable).replace("'", "''")
            run(["powershell", "-NoProfile", "-Command",
                 f"$s=Get-AuthenticodeSignature -LiteralPath '{literal}'; "
                 "if($s.Status -ne 'Valid' -or -not $s.TimeStamperCertificate){throw 'Invalid Authenticode signature'}"])
    else:
        manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    output.mkdir(parents=True, exist_ok=True)
    artifact = output / f"{app['package']}-{manifest['version']}-{platform}-{architecture}-{signing}-{manifest['source_commit'][:12]}.zip"
    if platform == "macos":
        run(["ditto", "-c", "-k", "--sequesterRsrc", "--keepParent", str(destination / f"{app['name']}.app"), str(artifact)])
    else:
        with zipfile.ZipFile(artifact, "w", zipfile.ZIP_DEFLATED) as package:
            for item in sorted(destination.rglob("*")):
                if item.is_file():
                    package.write(item, item.relative_to(destination))
    checksums(output, artifact, manifest)
    return artifact


def prepare_signing(destination: Path) -> None:
    manifest, path = stage_info(destination)
    if manifest["platform"] not in ("macos", "windows"):
        raise ValueError("Only macOS/Windows application signing is supported")
    manifest["signing"] = "signed"
    path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


def linux_package(destination: Path, output: Path) -> Path:
    config = configuration()
    app = config["application"]
    manifest, _ = stage_info(destination)
    platform = manifest["platform"]
    if platform not in ("debian", "fedora"):
        raise ValueError("Expected a Linux distro stage")
    os_release = Path("/etc/os-release").read_text()
    fields = dict(line.split("=", 1) for line in os_release.splitlines() if "=" in line)
    if fields.get("ID", "").strip('"') != platform or fields.get("VERSION_ID", "").strip('"') != config["targets"][platform]:
        raise ValueError("Build packages inside the configured native distro baseline")
    binary = destination / "usr/bin" / app["binary"]
    validate_binary(binary, platform, manifest["architecture"])
    output.mkdir(parents=True, exist_ok=True)
    values = dict(app, version=manifest["version"].replace("-", "~", 1))
    if platform == "debian":
        with tempfile.TemporaryDirectory(prefix="diir-shlibs-") as directory:
            working = Path(directory)
            (working / "debian").mkdir()
            (working / "debian/control").write_text(f"Source: {app['package']}\n\nPackage: {app['package']}\nArchitecture: any\n")
            shared = run(["dpkg-shlibdeps", "-O", f"-e{binary}"], cwd=working, capture_output=True).stdout
        generated = next((line.split("=", 1)[1] for line in shared.splitlines() if line.startswith("shlibs:Depends=")), "")
        if not generated:
            raise ValueError("dpkg-shlibdeps did not generate shared-library dependencies")
        values["depends"] = ", ".join([generated, *config["linux"]["debian_runtime"]])
        values["installed_size"] = (sum(item.stat().st_size for item in destination.rglob("*") if item.is_file()) + 1023) // 1024
        control = destination / "DEBIAN"
        control.mkdir()
        (control / "control").write_text((ROOT / "packaging/linux/control.in").read_text().format(**values), encoding="utf-8")
        artifact = output / f"{app['package']}_{values['version']}-{app['revision']}_amd64.deb"
        run(["dpkg-deb", "--root-owner-group", "--build", str(destination), str(artifact)])
        run(["dpkg-deb", "--info", str(artifact)])
        run(["dpkg-deb", "--contents", str(artifact)])
    else:
        with tempfile.TemporaryDirectory(prefix="diir-rpm-") as directory:
            top = Path(directory)
            for name in ("SOURCES", "SPECS", "BUILD", "BUILDROOT", "RPMS", "SRPMS"):
                (top / name).mkdir()
            shutil.copytree(destination, top / "SOURCES/payload")
            values["requires"] = ", ".join(config["linux"]["rpm_runtime"])
            spec = top / "SPECS/diir.spec"
            spec.write_text((ROOT / "packaging/linux/diir.spec.in").read_text().format(**values), encoding="utf-8")
            run(["rpmbuild", "--define", f"_topdir {top}", "-bb", str(spec)])
            rpms = list((top / "RPMS").rglob("*.rpm"))
            if len(rpms) != 1:
                raise ValueError("Expected exactly one generated RPM")
            artifact = output / rpms[0].name
            shutil.copy2(rpms[0], artifact)
        run(["rpm", "-qpi", str(artifact)])
        run(["rpm", "-qpl", str(artifact)])
        run(["rpm", "-qpR", str(artifact)])
    checksums(output, artifact, manifest)
    return artifact


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("config")
    create = commands.add_parser("stage")
    create.add_argument("--platform", choices=sorted({key[0] for key in TARGETS}), required=True)
    create.add_argument("--architecture", choices=["arm64", "x86_64"], required=True)
    create.add_argument("--binary", type=Path, required=True)
    create.add_argument("--output", type=Path, default=ROOT / "dist")
    for name in ("archive", "linux", "prepare-signing"):
        command = commands.add_parser(name)
        command.add_argument("--stage", type=Path, required=True)
        if name != "prepare-signing":
            command.add_argument("--output", type=Path, default=ROOT / "dist")
        if name == "archive":
            command.add_argument("--signing", choices=["unsigned", "signed"], default="unsigned")
    args = parser.parse_args()
    try:
        if args.command == "config":
            print(json.dumps(configuration(), indent=2))
        elif args.command == "stage":
            print(stage(args.platform, args.architecture, args.binary.resolve(), args.output))
        elif args.command == "prepare-signing":
            prepare_signing(args.stage)
        elif args.command == "archive":
            print(archive(args.stage.resolve(), args.output.resolve(), args.signing))
        else:
            print(linux_package(args.stage.resolve(), args.output.resolve()))
    except (ValueError, KeyError, OSError, subprocess.CalledProcessError) as error:
        print(f"Packaging failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
