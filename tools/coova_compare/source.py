"""Read immutable Git blobs and extract original function text, without edits."""

import hashlib
from pathlib import Path
import re
import subprocess

REVISION = "61854f91a5d159bdfe8a38794b99d86df81fddc7"
ROOT = Path(__file__).resolve().parents[2]
FUNCTIONS = ("dhcp_create_pkt", "dhcp_gettag", "dhcp_accept_opt",
             "dhcp_handler", "dhcp_sendOFFER")


def blob(repository, path):
    return subprocess.check_output(
        ["git", "-C", str(repository), "show", f"{REVISION}:{path}"], text=True,
    )


def function(source, name):
    pattern = rf"^(?:static[ \t]+)?(?:int|uint32_t|size_t)[ \t\n]+{name}\s*\("
    match = re.search(pattern, source, re.MULTILINE)
    if not match:
        raise ValueError(f"Missing original function: {name}")
    start = source.index("{", match.end())
    # Count braces outside C comments, strings and character literals.
    tokens = re.finditer(r'/\*.*?\*/|//[^\n]*|"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'|[{}]',
                         source[start:], re.DOTALL)
    depth = 0
    for token in tokens:
        if token.group() == "{":
            depth += 1
        elif token.group() == "}":
            depth -= 1
            if depth == 0:
                text = source[match.start():start + token.end()]
                return source[:match.start()].count("\n") + 1, text
    raise ValueError(f"Unterminated original function: {name}")


def prepare(repository, directory):
    files = {name: blob(repository, f"src/{name}")
             for name in ("dhcp.c", "dhcp.h", "pkt.h", "chksum.c")}
    directory.mkdir(parents=True, exist_ok=True)
    directory.joinpath("pkt.h").write_text(files["pkt.h"])
    # Preserve upstream notices in generated derivatives; nothing is vendored.
    notice = files["dhcp.c"].split('#include "system.h"')[0]
    constants = "\n".join(line for line in files["dhcp.h"].splitlines()
                          if re.match(r"#define (?:DHCP\w*|CHILLI_DHCP_\w+)\s+\d", line))
    directory.joinpath("constants.h").write_text(notice + constants + "\n")
    fragments = [notice, '#include "adapter.h"\n',
                 'static uint8_t bmac[6] = {255,255,255,255,255,255};\n',
                 '#define cksum_wrap(c) (c=(c>>16)+(c&0xffff),(~(c+(c>>16))&0xffff))\n']
    extracted = {}
    for filename, names in [("dhcp.c", FUNCTIONS), ("chksum.c", ("in_cksum", "chksum"))]:
        if filename == "chksum.c":
            fragments.append(files[filename].split('#include "chilli.h"')[0])
        for name in names:
            line, text = function(files[filename], name)
            fragments.append(f'\n#line {line} "coova/{filename}"\n{text}\n')
            extracted[name] = hashlib.sha256(text.encode()).hexdigest()
    directory.joinpath("original.c").write_text("".join(fragments))
    return {"revision": REVISION, "functions_sha256": extracted,
            "blobs_sha256": {name: hashlib.sha256(data.encode()).hexdigest()
                             for name, data in files.items()}}
