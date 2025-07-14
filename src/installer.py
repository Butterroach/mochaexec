#!/usr/bin/env python3

import platform
import sys
import urllib.request
import json
import os
import shutil

# for easy rebranding (you psychopaths can someone please take forking priviligies away from y'all)
BIN_PATH = "/bin/mchx"
GITHUB_REPO = "Butterroach/mochaexec"

if platform.system() != "Linux":
    print("this linux only gtfo")
    sys.exit(1)

arch = platform.machine().lower()
if arch in ("i386", "i686", "x86"):
    print("holy shit why are you still on 32bit")
    sys.exit(1)

arch_map = {"x86_64": "x86_64", "amd64": "x86_64", "aarch64": "arm64", "arm64": "arm64"}

if arch not in arch_map:
    print(
        f"why the actual hell are you on {arch} what the fuck is wrong with you what the fuck"
    )
    sys.exit(1)

target_arch = arch_map[arch]

api_url = f"https://api.github.com/repos/{GITHUB_REPO}/releases/latest"
try:
    with urllib.request.urlopen(api_url) as resp:
        release = json.load(resp)
except Exception as e:
    print(f"uwaaaaaa api req failedddx {e}")
    sys.exit(1)

asset_name_substr = f"-{target_arch}"
asset = None
for a in release["assets"]:
    if asset_name_substr in a["name"]:
        asset = a
        break

if not asset:
    print(
        f"no build found. open an issue at the github repo. complain. make noise. don't let me sleep."
    )
    sys.exit(1)

url = asset["browser_download_url"]
size = asset["size"]
print(f"downloading!!! {asset['name']} ({size} bytes)")

try:
    with urllib.request.urlopen(url) as resp, open("mchx.tmp", "wb") as out:
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
    print("\ndownload complete!!! :3")
except Exception as e:
    print(f"NOOOOOOO {e}")
    if os.path.exists("mchx.tmp"):
        os.remove("mchx.tmp")
    sys.exit(1)

try:
    shutil.move("mchx.tmp", BIN_PATH)
    os.chmod(BIN_PATH, 0o4755)
    print("installed successfully!!!!")
except Exception as e:
    print(f"wtf why cant i move to /bin what the fuk {e}")
    if os.path.exists("mchx.tmp"):
        os.remove("mchx.tmp")
    sys.exit(1)

adults_path = "/etc/mochaexec.d/responsible_adults"
os.makedirs(os.path.dirname(adults_path), exist_ok=True)

if not os.path.exists(adults_path):
    flags = os.O_WRONLY | os.O_CREAT | os.O_TRUNC
    mode = 0o644

    fd = os.open(adults_path, flags, mode)
    os.close(fd)

config_path = "/etc/mochaexec.d/config.toml"

if not os.path.exists(config_path):
    flags = os.O_WRONLY | os.O_CREAT | os.O_TRUNC
    mode = 0o644

    fd = os.open(config_path, flags, mode)
    os.close(fd)

    with open(config_path, "w") as f:
        f.write(
            """prompt = "{shorthand} {version} | {username} | password here!!!: "  # you can use the variables shorthand (mchx), name (mochaexec), version, and username
    prompt_start_color = [0, 220, 230]  # RGB, start of the gradient
    prompt_end_color = [220, 0, 220]  # RGB, end of the gradient (set this the same as the start color to disable gradients)
    """
        )
