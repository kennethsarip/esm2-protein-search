"""Corpus acquisition and manifest tests. Hand-computed, per CLAUDE.md 4.6."""

from __future__ import annotations

import json
import re
from pathlib import Path

import pytest

from esm2_search_eval.corpus import CorpusManifest, astral_40_url, sha256_file


def test_astral_40_url_pins_the_release_in_both_positions() -> None:
    """The release appears twice in a SCOPe URL: the directory and the filename.

    Interpolating only one of them is the failure that matters, because it
    does not 404. It silently serves a different release than the caller
    asked for, and R14 in CLAUDE.md 7.1 exists because two runs over
    different releases are not comparable. A benchmark that mixed 2.07
    sequences under a 2.08 label would look entirely normal.
    """
    assert astral_40_url("2.07") == (
        "https://scop.berkeley.edu/downloads/scopeseq-2.07/"
        "astral-scopedom-seqres-gd-sel-gs-bib-40-2.07.fa"
    )


# The NIST FIPS 180-4 test vector for SHA-256 over the three bytes "abc".
# Published, so it is an independently known expected value rather than
# whatever hashlib happens to return, which is what asserting against a
# hashlib call in the test would amount to.
SHA256_OF_ABC = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"


def test_sha256_file_matches_the_published_test_vector(tmp_path: Path) -> None:
    """Digesting a file holding exactly `abc` reproduces the NIST vector."""
    fasta = tmp_path / "corpus.fa"
    fasta.write_bytes(b"abc")
    assert sha256_file(fasta) == SHA256_OF_ABC


def manifest_for(sha256: str) -> CorpusManifest:
    """A manifest whose only field these tests vary is the pinned digest."""
    return CorpusManifest(
        release="2.08",
        url=astral_40_url("2.08"),
        sha256=sha256,
        n_domains=3,
        n_queries=2,
        downloaded_at="2026-08-25",
    )


def test_verify_accepts_the_file_the_manifest_was_built_from(tmp_path: Path) -> None:
    """A manifest pinned to the digest of `abc` verifies a file holding `abc`."""
    fasta = tmp_path / "corpus.fa"
    fasta.write_bytes(b"abc")
    manifest_for(SHA256_OF_ABC).verify(fasta)


def test_verify_rejects_a_corpus_swapped_under_the_same_release_label(
    tmp_path: Path,
) -> None:
    """Changing the bytes while the release string stays 2.08 must be fatal.

    This is R14 in concrete form. A release string alone cannot detect a
    re-downloaded, re-cut, or truncated corpus, and results computed over one
    corpus and reported against another are not wrong in any visible way. The
    digest is what makes the pin real rather than decorative.
    """
    fasta = tmp_path / "corpus.fa"
    fasta.write_bytes(b"abd")
    with pytest.raises(ValueError, match=re.escape("2.08")):
        manifest_for(SHA256_OF_ABC).verify(fasta)


def test_manifest_round_trips_through_a_json_file(tmp_path: Path) -> None:
    """The written JSON carries the release and digest, and reads back equal.

    The raw-key assertions matter as much as the round trip: a manifest that
    only this module can read is not a provenance record. D7 and any future
    rerun have to be able to open it and see which SCOPe release produced a
    results table.
    """
    path = tmp_path / "manifest.json"
    manifest = manifest_for(SHA256_OF_ABC)
    manifest.write(path)

    raw = json.loads(path.read_text())
    assert raw["release"] == "2.08"
    assert raw["sha256"] == SHA256_OF_ABC
    assert raw["n_queries"] == 2
    assert CorpusManifest.read(path) == manifest
