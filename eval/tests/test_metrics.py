"""Metric tests. Expected values are hand-computed, per CLAUDE.md 4.6."""

from __future__ import annotations

import pytest

from esm2_search_eval.metrics import average_precision, recall_at_k

# A five-item ranking whose answer is known by inspection.
#
#   rank:      1    2    3    4    5
#   candidate: d1   d2   d3   d4   d5
#   positive:  yes       yes       yes    -> 3 positives in total
RANKING = ["d1", "d2", "d3", "d4", "d5"]
RELEVANT = {"d1", "d3", "d5"}


def test_recall_at_1_divides_by_total_positives() -> None:
    """Top-1 is [d1]: one of the three positives, so 1/3.

    This case is the one that distinguishes the protocol's definition from the
    flattering one. Normalising by min(k, |R|) instead of |R| would report
    1/1 = 1.0 here, a perfect score for a ranking that found one positive out
    of three. If this assertion ever passes at 1.0, the denominator is wrong.
    """
    assert recall_at_k(RANKING, RELEVANT, 1) == pytest.approx(1 / 3)


def test_recall_at_3_counts_only_positives_inside_the_prefix() -> None:
    """Top-3 is [d1, d2, d3]. d1 and d3 are positives, d2 is not, so 2/3."""
    assert recall_at_k(RANKING, RELEVANT, 3) == pytest.approx(2 / 3)


def test_recall_at_5_recovers_every_positive() -> None:
    """The full ranking contains all three positives, so 3/3."""
    assert recall_at_k(RANKING, RELEVANT, 5) == pytest.approx(1.0)


def test_recall_at_k_larger_than_ranking_does_not_overcount() -> None:
    """k=100 against a 5-item ranking is still 3/3, not an error or >1.0.

    Real methods return short lists: PROTOCOL.md section 5 requires BLAST's
    empty positions to count as misses, which means recall@100 is routinely
    computed over far fewer than 100 results.
    """
    assert recall_at_k(RANKING, RELEVANT, 100) == pytest.approx(1.0)


def test_recall_at_k_rejects_a_query_with_no_positives() -> None:
    """Recall is undefined with an empty positive set.

    PROTOCOL.md section 3 drops such queries from the query set, so reaching
    this function with one is a bug in the harness, not a score of zero. A
    silent 0.0 would drag the mean down for every method and look like a
    quality problem rather than the loader bug it is.
    """
    with pytest.raises(ValueError, match="no positives"):
        recall_at_k(RANKING, set(), 10)


def test_average_precision_on_the_hand_computed_ranking() -> None:
    """Positives sit at ranks 1, 3, 5, so precision there is 1/1, 2/3, 3/5.

        AP = (1/1 + 2/3 + 3/5) / 3 = 2.2666... / 3 = 0.75555...

    The denominator is the number of positives, matching recall_at_k.
    """
    assert average_precision(RANKING, RELEVANT) == pytest.approx(0.7555555, abs=1e-6)


def test_average_precision_penalises_positives_ranked_lower() -> None:
    """Same positives, same recall@5, worse ordering, so a lower score.

    This is the case that gives average precision its reason to exist. Both
    rankings retrieve all three positives, so recall@5 is 1.0 for each and
    cannot tell them apart. Here the positives sit at ranks 3, 4, 5:

        AP = (1/3 + 2/4 + 3/5) / 3 = 1.43333... / 3 = 0.47777...

    An implementation that ignored rank order would return 0.7555 for both and
    this assertion would catch it.
    """
    worse = ["d2", "d4", "d1", "d3", "d5"]
    assert recall_at_k(worse, RELEVANT, 5) == pytest.approx(1.0)
    assert average_precision(worse, RELEVANT) == pytest.approx(0.4777777, abs=1e-6)
