"""Build the BLAST and MMseqs2 databases the alignment baselines search.

Both are built from one normalised FASTA written by `scope.write_fasta`, not
from the downloaded file. PROTOCOL.md section 5 requires every method to see
an identical corpus, and the only way to be sure of that is for every method
to read bytes this harness produced.
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

from esm2_search_eval.corpus import DEFAULT_RELEASE, CorpusManifest
from esm2_search_eval.scope import parse_astral_fasta, write_fasta


def tool_versions() -> dict[str, str]:
    """Versions of every external binary, for the results manifest.

    PROTOCOL.md section 6 commits to reporting these beside every table. A
    baseline is only a baseline if a reader can tell which build produced it.
    """
    return {
        "blast": _run(["blastp", "-version"]).splitlines()[0].split(":", 1)[1].strip(),
        "mmseqs": _run(["mmseqs", "version"]).strip(),
    }


def _run(command: list[str]) -> str:
    """Run a command, returning stdout and raising with stderr on failure."""
    result = subprocess.run(command, capture_output=True, text=True, check=False)
    if result.returncode != 0:
        raise RuntimeError(f"{command[0]} failed: {result.stderr.strip()[:400]}")
    return result.stdout


def build_blast_db(fasta: Path, out_prefix: Path) -> None:
    """Index the corpus for blastp, keeping the ASTRAL sid as the sequence id."""
    _run(
        [
            "makeblastdb",
            "-in",
            str(fasta),
            "-dbtype",
            "prot",
            "-parse_seqids",
            "-out",
            str(out_prefix),
        ]
    )


def build_mmseqs_db(fasta: Path, out_prefix: Path) -> None:
    """Index the same corpus for mmseqs search."""
    _run(["mmseqs", "createdb", str(fasta), str(out_prefix)])


def main() -> None:
    """Verify the pinned corpus, normalise it, and build both databases."""
    release = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_RELEASE
    data = Path(__file__).resolve().parents[2] / "data"

    manifest = CorpusManifest.read(data / f"manifest-{release}.json")
    source = data / f"astral-scopedom-seqres-gd-sel-gs-bib-40-{release}.fa"
    manifest.verify(source)

    domains = parse_astral_fasta(source.read_text())
    fasta = data / f"corpus-{release}.fa"
    write_fasta(domains, fasta)
    print(f"normalised {len(domains)} domains to {fasta.name}", file=sys.stderr)

    db_dir = data / "db"
    db_dir.mkdir(exist_ok=True)
    build_blast_db(fasta, db_dir / f"blast-{release}")
    print("blast db built", file=sys.stderr)
    build_mmseqs_db(fasta, db_dir / f"mmseqs-{release}")
    print("mmseqs db built", file=sys.stderr)

    versions = tool_versions()
    (data / f"tools-{release}.json").write_text(
        json.dumps(versions, indent=2, sort_keys=True) + "\n"
    )
    print(f"blast {versions['blast']}, mmseqs {versions['mmseqs']}", file=sys.stderr)


if __name__ == "__main__":
    main()
