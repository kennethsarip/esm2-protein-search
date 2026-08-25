"""SCOPe parsing tests. Expected values are hand-computed, per CLAUDE.md 4.6."""

from __future__ import annotations

import pytest

from esm2_search_eval.scope import (
    Domain,
    candidate_pool,
    is_positive,
    parse_astral_fasta,
    parse_sccs,
    positives,
    query_set,
)


def test_parse_sccs_splits_the_hierarchy_at_each_level() -> None:
    """`a.1.1.2` is class a, fold a.1, superfamily a.1.1, family a.1.1.2.

    PROTOCOL.md section 2 rests the positive definition on superfamily and
    family, so each level has to be the cumulative prefix and not the bare
    component: the superfamily of `a.1.1.2` is `a.1.1`, never `1`. Comparing
    bare components would make `a.1.1` and `b.4.1` share a superfamily.
    """
    sccs = parse_sccs("a.1.1.2")
    assert sccs.klass == "a"
    assert sccs.fold == "a.1"
    assert sccs.superfamily == "a.1.1"
    assert sccs.family == "a.1.1.2"


@pytest.mark.parametrize("malformed", ["a.1.1", "a.1.1.2.3", "a", ""])
def test_parse_sccs_rejects_a_string_that_is_not_four_levels(malformed: str) -> None:
    """A three-level sccs must not parse into a family equal to its superfamily.

    Silently accepting `a.1.1` would set family == superfamily == `a.1.1`, and
    the positive rule in PROTOCOL.md section 2 (same superfamily, different
    family) would then reject every genuine positive in that superfamily. The
    benchmark would report a low recall for every method and look like a
    quality result rather than the parsing bug it is.
    """
    with pytest.raises(ValueError, match="four levels"):
        parse_sccs(malformed)


# Two records in the ASTRAL header layout, hand-written so every expected value
# below is known by inspection:
#
#   >sid sccs (descriptor) description {species}
#
# d1dlwa_ has its sequence wrapped across two lines and carries a lowercase
# residue; d2xyzb1 is a single line.
ASTRAL_FASTA = """>d1dlwa_ a.1.1.1 (A:) Protoglobin {Methanosarcina acetivorans [TaxId: 188937]}
MSLFAK
lGGRE
>d2xyzb1 b.4.1.2 (B:1-40) Fake domain {Escherichia coli}
KTAYIA
"""


def test_parse_astral_fasta_reads_sid_sccs_and_sequence() -> None:
    """The first record's sequence spans two lines and must join to one string.

    The lowercase `l` is uppercased deliberately. blastp does not mask
    lowercase by default, measured on 2.17.0+, so this is not fixing a live
    bug. It is normalisation: under `-lcase_masking` a fully lowercase corpus
    scores zero hits, and uppercasing at load makes it impossible for any
    runner to trip that flag into the silent corpus difference PROTOCOL.md
    section 5 forbids.
    """
    domains = parse_astral_fasta(ASTRAL_FASTA)
    assert len(domains) == 2
    assert domains[0].sid == "d1dlwa_"
    assert domains[0].sccs.superfamily == "a.1.1"
    assert domains[0].sequence == "MSLFAKLGGRE"
    assert domains[1].sid == "d2xyzb1"
    assert domains[1].sccs.family == "b.4.1.2"
    assert domains[1].sequence == "KTAYIA"


def domain(sid: str, sccs: str) -> Domain:
    """A domain with a sequence no test inspects, to keep the sccs the subject."""
    return Domain(sid=sid, sccs=parse_sccs(sccs), sequence="MKTAYIA")


# A toy corpus covering every branch of PROTOCOL.md section 2, relative to Q:
#
#   sid          sccs       relation to Q                      role
#   Q            a.1.1.1    itself                             excluded
#   SAME_FAMILY  a.1.1.1    same superfamily, same family      excluded
#   POS_1        a.1.1.2    same superfamily, other family     positive
#   POS_2        a.1.1.5    same superfamily, other family     positive
#   FOLD_ONLY    a.1.2.1    same fold, other superfamily       negative
#   FAR          b.4.1.2    unrelated                          negative
Q = domain("Q", "a.1.1.1")
SAME_FAMILY = domain("SAME_FAMILY", "a.1.1.1")
POS_1 = domain("POS_1", "a.1.1.2")
POS_2 = domain("POS_2", "a.1.1.5")
FOLD_ONLY = domain("FOLD_ONLY", "a.1.2.1")
FAR = domain("FAR", "b.4.1.2")
CORPUS = [Q, SAME_FAMILY, POS_1, POS_2, FOLD_ONLY, FAR]


def test_is_positive_requires_the_same_superfamily_and_a_different_family() -> None:
    """Same superfamily alone is not enough; the family must differ.

    SAME_FAMILY is the case that matters. Treating it as a positive would let
    a method score on same-family retrieval, which is the easy problem
    alignment already solves and which section 1 of CLAUDE.md says we are
    explicitly looking past. FOLD_ONLY is a negative by the conservative
    reading in PROTOCOL.md section 2.
    """
    assert is_positive(Q, POS_1) is True
    assert is_positive(Q, POS_2) is True
    assert is_positive(Q, SAME_FAMILY) is False
    assert is_positive(Q, FOLD_ONLY) is False
    assert is_positive(Q, FAR) is False


def test_candidate_pool_drops_the_query_and_its_whole_family() -> None:
    """Q's pool is POS_1, POS_2, FOLD_ONLY, FAR, in corpus order.

    SAME_FAMILY occupies no ranking position at all. PROTOCOL.md section 2
    removes it rather than scoring it as a negative: if it stayed in the pool
    every method would be penalised for ranking a near-identical domain
    highly, which is behaviour we do not want to measure in either direction.
    """
    pool = candidate_pool(Q, CORPUS)
    assert [d.sid for d in pool] == ["POS_1", "POS_2", "FOLD_ONLY", "FAR"]


def test_positives_returns_only_the_remote_homologs_of_the_query() -> None:
    """Q's positives are POS_1 and POS_2, the two same-superfamily other families.

    This set is the `relevant` argument the metrics in `metrics.py` divide by,
    so an error here scales every recall number in the benchmark. FOLD_ONLY
    sits in the pool but is not relevant; SAME_FAMILY is in neither.
    """
    assert positives(Q, CORPUS) == {"POS_1", "POS_2"}


def test_query_set_drops_domains_that_have_no_possible_positive() -> None:
    """FOLD_ONLY and FAR are alone in their superfamilies, so they cannot query.

    PROTOCOL.md section 3 drops them because recall is undefined with an empty
    positive set. Keeping them would make `recall_at_k` raise, and answering
    0.0 instead would divide every method's mean by the same arbitrary
    constant and read as a quality difference between methods that is not one.
    """
    assert [d.sid for d in query_set(CORPUS)] == ["Q", "SAME_FAMILY", "POS_1", "POS_2"]
