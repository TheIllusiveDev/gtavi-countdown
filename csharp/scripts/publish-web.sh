#!/usr/bin/env bash
# Publishes the web app as a static site and finishes its response headers.
#
# The build writes an import map inline into index.html. The Content Security
# Policy in _headers allows exactly that script by its hash, which is only
# known after the build, so it is filled in here.
#
# Usage: scripts/publish-web.sh [output-directory]   (default: dist)
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
out="${1:-$root/dist}"

dotnet publish "$root/src/GtaviCountdown.Web" -c Release -o "$out/.publish"
rm -rf "$out/site"
mv "$out/.publish/wwwroot" "$out/site"
rm -rf "$out/.publish"

python3 - "$out/site" <<'PY'
import base64, hashlib, pathlib, re, sys

site = pathlib.Path(sys.argv[1])
html = (site / "index.html").read_text(encoding="utf-8")
match = re.search(r'<script type="importmap">(.*?)</script>', html, re.S)
if not match:
    sys.exit("index.html has no inline import map")
digest = base64.b64encode(hashlib.sha256(match.group(1).encode("utf-8")).digest()).decode()
headers = site / "_headers"
headers.write_text(headers.read_text(encoding="utf-8").replace("sha256-IMPORTMAP", f"sha256-{digest}"), encoding="utf-8")
print(f"Import map hash: sha256-{digest}")
PY

echo "Static site written to $out/site"
