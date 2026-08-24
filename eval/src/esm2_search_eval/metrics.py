"""Ranking metrics for the remote-homology benchmark.

Every definition here is fixed in eval/PROTOCOL.md section 4. If a metric here
and the protocol ever disagree, the protocol wins and this module is the bug.
"""

from __future__ import annotations

from collections.abc import Sequence


def recall_at_k(ranking: Sequence[str], relevant: set[str], k: int) -> float:
    """Fraction of all positives that appear in the top k of `ranking`.

    Raises:
        ValueError: if `relevant` is empty, which PROTOCOL.md section 3
            excludes from the query set. Returning 0.0 instead would depress
            every method's mean identically and read as a quality problem
            rather than the harness bug it is.
    """
    if not relevant:
        raise ValueError("recall is undefined for a query with no positives")
    hits = sum(1 for candidate in ranking[:k] if candidate in relevant)
    return hits / len(relevant)


def average_precision(ranking: Sequence[str], relevant: set[str]) -> float:
    """Mean of the precision measured at each position holding a positive.

    Positives the ranking never returns contribute nothing to the numerator
    but still count in the denominator, so a method is charged for what it
    failed to retrieve. PROTOCOL.md section 5.

    Raises:
        ValueError: if `relevant` is empty. See `recall_at_k`.
    """
    if not relevant:
        raise ValueError("average precision is undefined for a query with no positives")

    hits = 0
    precision_sum = 0.0
    for position, candidate in enumerate(ranking, start=1):
        if candidate in relevant:
            hits += 1
            precision_sum += hits / position
    return precision_sum / len(relevant)
