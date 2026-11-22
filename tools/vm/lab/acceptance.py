"""Versioned target metadata; the one-client functional run cannot pass scale."""

import hashlib
from pathlib import Path
import tomllib


def load():
    path = Path(__file__).with_name("acceptance.toml")
    contents = path.read_bytes()
    profile = tomllib.loads(contents.decode("utf-8"))
    if profile.get("version") != 1 or profile.get("id") != "mvp-lab-v1":
        raise ValueError("Unsupported lab acceptance profile")
    return {"sha256": hashlib.sha256(contents).hexdigest(), "profile": profile}
