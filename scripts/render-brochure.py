"""Print the self-contained two-page brochure with an installed headless browser."""

from pathlib import Path
import shutil
import subprocess
import tempfile

root = Path(__file__).resolve().parents[1]
source = root / "docs/irongate-performance-brochure.html"
output = source.with_suffix(".pdf")
browser = next((shutil.which(name) for name in ("google-chrome", "chromium", "chromium-browser")
                if shutil.which(name)), None)
if browser is None:
    raise SystemExit("An installed Chrome/Chromium browser is required to render the brochure.")
with tempfile.TemporaryDirectory(prefix="irongate-brochure-") as profile:
    subprocess.run([browser, "--headless", "--disable-gpu", "--disable-background-networking",
                    "--disable-extensions", "--disable-component-update", "--no-first-run",
                    f"--user-data-dir={profile}", "--no-pdf-header-footer", f"--print-to-pdf={output}",
                    source.as_uri()], check=True)
print(output)
