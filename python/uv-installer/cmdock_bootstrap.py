from __future__ import annotations

import io
import json
import os
import platform
import shutil
import subprocess
import sys
import tarfile
import tempfile
import urllib.request
import zipfile
from pathlib import Path

REPO = os.environ.get("CMDOCK_REPO", "torifo/cmd-mock-cli")
BIN_NAME = "cmdock"
LEGACY_BIN_NAME = "cmd-mock-cli"
API_BASE = f"https://api.github.com/repos/{REPO}"


def main() -> int:
    version = os.environ.get("CMDOCK_VERSION") or latest_release_tag()
    binary_path = install_binary(version)
    process = subprocess.run([str(binary_path), *sys.argv[1:]], check=False)
    return process.returncode


def latest_release_tag() -> str:
    request = urllib.request.Request(
        f"{API_BASE}/releases/latest",
        headers={"Accept": "application/vnd.github+json"},
    )
    with urllib.request.urlopen(request) as response:
        payload = json.load(response)
    tag_name = payload.get("tag_name")
    if not tag_name:
        raise SystemExit("Failed to determine the latest cmdock release tag")
    return str(tag_name)


def install_binary(version: str) -> Path:
    install_root = data_home() / "cmdock" / "releases" / version
    binary_name = binary_filename()
    target = install_root / binary_name
    if target.exists():
        target.chmod(0o755)
        return target

    install_root.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="cmdock-uv-") as temp_dir:
        archive_path = download_release_archive(version, Path(temp_dir))
        unpack_archive(archive_path, Path(temp_dir))
        extracted = extracted_binary(Path(temp_dir), binary_name)
        if not extracted.exists():
            raise SystemExit(f"Binary {binary_name} not found in {archive_path.name}")
        shutil.move(str(extracted), str(target))
        target.chmod(0o755)

    return target


def release_asset_names() -> list[str]:
    os_tag = os_release_tag()
    arch_tag = arch_release_tag()
    if os_tag == "windows":
        extension = ".zip"
    else:
        extension = ".tar.gz"
    return [
        f"{BIN_NAME}-{os_tag}-{arch_tag}{extension}",
        f"{LEGACY_BIN_NAME}-{os_tag}-{arch_tag}{extension}",
    ]


def binary_filename() -> str:
    if platform.system().lower().startswith("win"):
        return f"{BIN_NAME}.exe"
    return BIN_NAME


def os_release_tag() -> str:
    system = platform.system().lower()
    if system == "darwin":
        return "macos"
    if system == "linux":
        return "linux"
    if system.startswith("win"):
        return "windows"
    raise SystemExit(f"Unsupported OS: {system}")


def arch_release_tag() -> str:
    machine = platform.machine().lower()
    if machine in {"x86_64", "amd64"}:
        return "x86_64"
    if machine in {"aarch64", "arm64"}:
        return "aarch64"
    raise SystemExit(f"Unsupported architecture: {machine}")


def data_home() -> Path:
    if os.name == "nt":
        base = os.environ.get("LOCALAPPDATA")
        if base:
            return Path(base)
    xdg = os.environ.get("XDG_DATA_HOME")
    if xdg:
        return Path(xdg)
    return Path.home() / ".local" / "share"


def download_release_archive(version: str, temp_dir: Path) -> Path:
    last_error: Exception | None = None
    for asset_name in release_asset_names():
        destination = temp_dir / asset_name
        url = f"https://github.com/{REPO}/releases/download/{version}/{asset_name}"
        try:
            download(url, destination)
            return destination
        except Exception as error:
            last_error = error
    raise SystemExit(
        f"Failed to download a compatible release archive for {version}: {last_error}"
    )


def download(url: str, destination: Path) -> None:
    request = urllib.request.Request(
        url,
        headers={"Accept": "application/octet-stream"},
    )
    with urllib.request.urlopen(request) as response:
        destination.write_bytes(response.read())


def unpack_archive(archive_path: Path, destination: Path) -> None:
    if archive_path.suffix == ".zip":
        with zipfile.ZipFile(archive_path) as archive:
            archive.extractall(destination)
        return

    if "".join(archive_path.suffixes[-2:]) == ".tar.gz":
        with tarfile.open(fileobj=io.BytesIO(archive_path.read_bytes()), mode="r:gz") as archive:
            archive.extractall(destination)
        return

    raise SystemExit(f"Unsupported archive format: {archive_path.name}")


def extracted_binary(destination: Path, binary_name: str) -> Path:
    direct = destination / binary_name
    if direct.exists():
        return direct

    legacy = destination / legacy_binary_filename()
    if legacy.exists():
        return legacy

    return direct


def legacy_binary_filename() -> str:
    if platform.system().lower().startswith("win"):
        return f"{LEGACY_BIN_NAME}.exe"
    return LEGACY_BIN_NAME
