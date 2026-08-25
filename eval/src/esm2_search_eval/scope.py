"""SCOPe ASTRAL corpus loading and the sccs hierarchy.

The positive definition this module serves is fixed in eval/PROTOCOL.md
section 2. Where this module and the protocol disagree, the protocol wins.
"""

from __future__ import annotations

from collections import defaultdict
from collections.abc import Sequence
from dataclasses import dataclass


@dataclass(frozen=True)
class Sccs:
    """The four levels of a SCOPe classification string."""

    klass: str
    fold: str
    superfamily: str
    family: str


def parse_sccs(sccs: str) -> Sccs:
    """Split a SCOPe `sccs` string into its cumulative hierarchy prefixes.

    Raises:
        ValueError: if `sccs` does not have exactly four levels. See the test
            for why this is fatal rather than tolerated.
    """
    parts = sccs.split(".")
    if len(parts) != 4 or not all(parts):
        raise ValueError(f"sccs must have four levels, got {sccs!r}")
    return Sccs(
        klass=parts[0],
        fold=".".join(parts[:2]),
        superfamily=".".join(parts[:3]),
        family=".".join(parts[:4]),
    )


@dataclass(frozen=True)
class Domain:
    """One ASTRAL record: its stable identifier, classification, and sequence."""

    sid: str
    sccs: Sccs
    sequence: str


def parse_astral_fasta(text: str) -> list[Domain]:
    """Read ASTRAL FASTA text into domains, one per record.

    Raises:
        ValueError: if a header carries fewer than the sid and sccs fields, or
            if a record has no sequence.
    """
    domains: list[Domain] = []
    sid = ""
    sccs = ""
    residues: list[str] = []

    def flush() -> None:
        if not sid:
            return
        if not residues:
            raise ValueError(f"record {sid!r} has no sequence")
        domains.append(Domain(sid=sid, sccs=parse_sccs(sccs), sequence="".join(residues)))

    for line in text.splitlines():
        if line.startswith(">"):
            flush()
            fields = line[1:].split(maxsplit=2)
            if len(fields) < 2:
                raise ValueError(f"header lacks a sid and an sccs: {line!r}")
            sid, sccs = fields[0], fields[1]
            residues = []
        elif line.strip():
            residues.append(line.strip().upper())
    flush()
    return domains


def is_positive(query: Domain, candidate: Domain) -> bool:
    """Whether `candidate` is a remote homolog of `query`: PROTOCOL.md section 2."""
    return (
        query.sccs.superfamily == candidate.sccs.superfamily
        and query.sccs.family != candidate.sccs.family
    )


def candidate_pool(query: Domain, corpus: Sequence[Domain]) -> list[Domain]:
    """Everything `query` is ranked against: PROTOCOL.md section 3."""
    return [d for d in corpus if d.sid != query.sid and d.sccs.family != query.sccs.family]


def positives(query: Domain, corpus: Sequence[Domain]) -> set[str]:
    """The sids in `query`'s pool that are remote homologs of it."""
    return {d.sid for d in candidate_pool(query, corpus) if is_positive(query, d)}


def query_set(corpus: Sequence[Domain]) -> list[Domain]:
    """Domains that can be queried at all: PROTOCOL.md section 3.

    Deterministic and total. There is no sampling, so two runs over one SCOPe
    release produce the same query set in the same order.
    """
    families_by_superfamily: dict[str, set[str]] = defaultdict(set)
    for d in corpus:
        families_by_superfamily[d.sccs.superfamily].add(d.sccs.family)
    return [d for d in corpus if len(families_by_superfamily[d.sccs.superfamily]) > 1]
