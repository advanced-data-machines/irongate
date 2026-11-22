"""Fetch/cache a checksum-pinned base image; never boots or configures networking."""

import hashlib
from pathlib import Path
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[2]
ARTIFACTS = ROOT / "target/vm"


def specification():
    with Path(__file__).with_name("image.toml").open("rb") as source:
        return tomllib.load(source)


def digest(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha512").hexdigest()


def verify(path, expected):
    if digest(path) != expected:
        raise ValueError(f"Image checksum mismatch: {path}; it will not be used.")


def fetch():
    config = specification()
    directory = ARTIFACTS / "images"
    directory.mkdir(parents=True, exist_ok=True)
    image = directory / config["filename"]
    if image.exists():
        verify(image, config["sha512"])
    else:
        with tempfile.TemporaryDirectory(prefix="download-", dir=directory) as temporary:
            partial = Path(temporary) / config["filename"]
            subprocess.run(["curl", "--fail", "--location", "--proto", "=https",
                            "--proto-redir", "=https", "--connect-timeout", "15",
                            "--max-time", "600", "--output", str(partial), config["url"]], check=True)
            verify(partial, config["sha512"])
            partial.chmod(0o444)
            partial.replace(image)
    print(f"Verified read-only Debian base: {image}", flush=True)
    return image


if __name__ == "__main__":
    fetch()
