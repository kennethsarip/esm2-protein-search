"""SCOPe ASTRAL corpus acquisition and the manifest that pins it.

PROTOCOL.md section 1 requires the exact SCOPe release string beside every
results file, for the reason R14 pins the UniProt release: two runs against
different releases are never compared.
"""

from __future__ import annotations

import hashlib
import json
import sys
from dataclasses import asdict, dataclass
from datetime import date
from pathlib import Path

import httpx

from esm2_search_eval.scope import parse_astral_fasta, query_set

ASTRAL_BASE = "https://scop.berkeley.edu/downloads"
DEFAULT_RELEASE = "2.08"


def astral_40_url(release: str) -> str:
    """The ASTRAL SEQRES 40 percent identity subset for a SCOPe release."""
    return f"{ASTRAL_BASE}/scopeseq-{release}/astral-scopedom-seqres-gd-sel-gs-bib-40-{release}.fa"


def sha256_file(path: Path) -> str:
    """Hex SHA-256 of a file, read in chunks so a full corpus never lands in RAM."""
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        while chunk := handle.read(1 << 20):
            digest.update(chunk)
    return digest.hexdigest()


@dataclass(frozen=True)
class CorpusManifest:
    """What every results file must carry beside it: PROTOCOL.md section 1."""

    release: str
    url: str
    sha256: str
    n_domains: int
    n_queries: int
    downloaded_at: str

    def verify(self, path: Path) -> None:
        """Check `path` still holds the bytes this manifest was built from.

        Raises:
            ValueError: on any digest mismatch.
        """
        actual = sha256_file(path)
        if actual != self.sha256:
            raise ValueError(
                f"{path} does not match the corpus pinned for SCOPe {self.release}: "
                f"manifest {self.sha256}, file {actual}"
            )

    def write(self, path: Path) -> None:
        """Serialise beside a results file."""
        path.write_text(json.dumps(asdict(self), indent=2, sort_keys=True) + "\n")

    @classmethod
    def read(cls, path: Path) -> CorpusManifest:
        """Load a manifest written by `write`."""
        return cls(**json.loads(path.read_text()))


def download_astral_40(release: str, dest_dir: Path) -> Path:
    """Fetch the ASTRAL 40 subset, skipping the fetch when it is already local.

    The download lands on a `.partial` file and is renamed into place only
    once complete. CLAUDE.md 4.5 requires resumability, and the failure it
    guards against here is a half-written FASTA that a rerun would treat as
    finished: the corpus would simply be short, and every method would score
    against it without complaint.
    """
    dest_dir.mkdir(parents=True, exist_ok=True)
    path = dest_dir / f"astral-scopedom-seqres-gd-sel-gs-bib-40-{release}.fa"
    if path.exists():
        return path

    partial = path.with_suffix(".partial")
    with httpx.stream(
        "GET", astral_40_url(release), follow_redirects=True, timeout=120.0
    ) as response:
        response.raise_for_status()
        with partial.open("wb") as handle:
            for chunk in response.iter_bytes(1 << 20):
                handle.write(chunk)
    partial.rename(path)
    return path


def main() -> None:
    """Download a SCOPe release, summarise it, and pin it in a manifest."""
    release = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_RELEASE
    dest = Path(__file__).resolve().parents[2] / "data"

    print(f"fetching SCOPe {release} ASTRAL 40", file=sys.stderr)
    fasta = download_astral_40(release, dest)
    domains = parse_astral_fasta(fasta.read_text())
    queries = query_set(domains)

    manifest = CorpusManifest(
        release=release,
        url=astral_40_url(release),
        sha256=sha256_file(fasta),
        n_domains=len(domains),
        n_queries=len(queries),
        downloaded_at=date.today().isoformat(),
    )
    manifest.write(dest / f"manifest-{release}.json")
    print(
        f"{manifest.n_domains} domains, {manifest.n_queries} queryable, "
        f"sha256 {manifest.sha256[:12]}",
        file=sys.stderr,
    )


if __name__ == "__main__":
    main()
