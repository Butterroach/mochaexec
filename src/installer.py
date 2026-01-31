#!/usr/bin/env python3

"""
mochaexec: sudo if it was actually good. for responsible adults only.
Copyright (C) 2025-2026 Butterroach

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
"""

import hashlib
import json
import os
import platform
import shutil
import sys
import tempfile
import urllib.request

# for easy rebranding (you psychopaths stop forking everything
# can someone please take forking privileges away from y'all)
BIN_PATH = "/bin/mchx"
GITHUB_REPO = "Butterroach/mochaexec"
CONFIG_DIR = "/etc/mochaexec.d"
PROJECT_NAME = "mochaexec"
PROJECT_SHORTHAND = "mchx"

if platform.system() != "Linux":
    print(
        "this linux only gtfo",
        file=sys.stderr,
    )
    sys.exit(1)

import pwd

arch = platform.machine().lower()
if arch in ("i386", "i686", "x86"):
    print(
        "holy shit why are you still on 32bit",
        file=sys.stderr,
    )
    sys.exit(1)

arch_map = {"x86_64": "x86_64", "amd64": "x86_64", "aarch64": "arm64", "arm64": "arm64"}

if arch not in arch_map:
    print(
        f"why the actual hell are you on {arch} what the fuck is wrong with you what the fuck",
        file=sys.stderr,
    )
    sys.exit(1)

if os.geteuid() != 0:
    print(
        "you must run this script as root IDIOT",
        file=sys.stderr,
    )
    sys.exit(1)

target_arch = arch_map[arch]

api_url = f"https://api.github.com/repos/{GITHUB_REPO}/releases/latest"
try:
    with urllib.request.urlopen(api_url) as resp:
        release = json.load(resp)
except Exception as e:
    print(
        f"uwaaaaaa api req failedddx {e}",
        file=sys.stderr,
    )
    sys.exit(1)

asset_name_substr = f"-{target_arch}"
asset = None
for a in release["assets"]:
    if asset_name_substr in a["name"]:
        asset = a
        break

if not asset:
    print(
        f"no build found. open an issue at the github repo. complain. make noise. don't let me sleep.",
        file=sys.stderr,
    )
    sys.exit(1)

url: str = asset["browser_download_url"]
size: int = asset["size"]
print(f"downloading!!! {asset['name']} ({size} bytes)")

fd, path = tempfile.mkstemp(dir="/tmp")

try:
    with urllib.request.urlopen(url) as resp, os.fdopen(fd, "wb") as out:
        downloaded = 0
        chunk_size = 8192
        while True:
            chunk = resp.read(chunk_size)
            if not chunk:
                break
            out.write(chunk)
            downloaded += len(chunk)
            percent = int(downloaded / size * 100)
            bar = ("#" * (percent // 2)).ljust(50)
            print(f"\r[{bar}] {percent:3d}%", end="", flush=True)
        out.flush()
        os.fsync(out.fileno())
    print("\ndownload complete!!! :3")
except Exception as e:
    print(
        f"NOOOOOOO {e}",
        file=sys.stderr,
    )
    if os.path.exists(path):
        os.remove(path)
    sys.exit(1)

with open(path, "rb") as f:
    binary_hash = hashlib.sha256(f.read())

if "sha256:" + binary_hash.hexdigest() != asset["digest"]:
    print(
        "THE SCARY BOOGEYMAN ON UR NETWORK TRIED TO HACK U AHHHH\n(failed to verify integrity!)",
        file=sys.stderr,
    )
    if os.path.exists(path):
        os.remove(path)
    sys.exit(1)

try:
    shutil.move(path, BIN_PATH)
    os.chmod(BIN_PATH, 0o4755)
except Exception as e:
    print(
        f"wtf why cant i move to /bin what the fuk {e}",
        file=sys.stderr,
    )
    if os.path.exists(path):
        os.remove(path)
    sys.exit(1)

adults_path = f"{CONFIG_DIR}/responsible_adults"
os.makedirs(os.path.dirname(adults_path), exist_ok=True)

if not os.path.exists(adults_path):
    flags = os.O_WRONLY | os.O_CREAT | os.O_TRUNC
    mode = 0o644

    fd = os.open(adults_path, flags, mode)
    os.close(fd)

    if os.getuid() != os.geteuid() or os.environ.get("SUDO_USER"):
        if (
            input("add your user to list of responsible adults (y/N)? ")
            .casefold()
            .startswith("y")
        ):
            with open(adults_path, "w") as f:
                if os.environ.get("SUDO_USER"):
                    f.write(os.environ.get("SUDO_USER") + "\n")
                else:
                    f.write(pwd.getpwuid(os.getuid()).pw_name + "\n")
    else:
        print(f"""can't figure out your user!!!!! >_<
you will have to add your username to {adults_path} manually!!!""")

config_path = f"{CONFIG_DIR}/config.toml"

if not os.path.exists(config_path):
    flags = os.O_WRONLY | os.O_CREAT | os.O_TRUNC
    mode = 0o644

    fd = os.open(config_path, flags, mode)
    os.close(fd)

    with open(config_path, "w") as f:
        f.write(
            """prompt = "{shorthand} {version} | {username} | password here!!!: "  # you can use the variables shorthand ([SHORTHAND]), name ([NAME]), version, and username
prompt_start_color = [0, 220, 230]  # RGB, start of the gradient
prompt_end_color = [220, 0, 220]  # RGB, end of the gradient (set this the same as the start color to disable gradients)
""".replace(
                "[SHORTHAND]", PROJECT_SHORTHAND
            ).replace(
                "[NAME]", PROJECT_NAME
            )
        )

print("\adone installing!! :3")
print("try out your installation now by running:")
print(f"\t{BIN_PATH.split('/')[-1]} whoami")
print(f"{PROJECT_NAME} has been installed in {BIN_PATH}")
print(
    f"customize your prompt at {config_path} (if this config is broken, {PROJECT_NAME} will tell you and default! dw)"
)
print(
    f"edit the responsible adults list (basically sudoers) at {adults_path} (line-separated usernames!)"
)
print("enjoy!!!! to update, just re-run this script! your config won't be lost")
