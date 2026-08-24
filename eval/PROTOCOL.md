# Evaluation protocol

Fixed in writing before any results were produced, per CLAUDE.md WS-D task
D5.5. Choosing a metric after seeing results is how honest people produce
dishonest benchmarks, so every choice below is committed to now, while we
still have no idea which method wins.

Anything in this document may be changed only in its own commit, with the
reasoning stated and the date recorded in the changelog at the bottom. A
change made after results exist must say so explicitly.

## 1. Corpus

SCOPe ASTRAL, 40 percent sequence identity subset. Q2 in CLAUDE.md 7.3 chose
this as a self-contained corpus rather than mapping SCOPe domains onto their
parent Swiss-Prot entries.

The 40 percent subset matters. At higher identity thresholds the corpus is
full of near-duplicates, and retrieval becomes a test of finding obvious
matches, which alignment already solves. Filtering to 40 percent is what makes
the remaining same-superfamily pairs genuinely remote.

The exact SCOPe release string is pinned in the manifest written alongside
every results file. Two runs against different releases are never compared,
for the same reason R14 pins the UniProt release.

## 2. The hierarchy, and what counts as a positive

SCOPe classifies each domain as class, fold, superfamily, family, encoded in
the `sccs` string (for example `a.1.1.2`: class `a`, fold `a.1`, superfamily
`a.1.1`, family `a.1.1.2`).

For a query domain q and a candidate domain c:

- **Positive**: same superfamily, different family. This is the definition of
  remote homology the whole project rests on. These are proteins that share an
  evolutionary origin but have diverged far enough that sequence alignment
  struggles.
- **Excluded**: same family. Removed from the candidate pool for that query
  entirely, so they occupy no ranking position and can neither help nor hurt
  any method. Same-family retrieval is the easy case; leaving it in would let
  a method score well on exactly the problem we claim to be looking past.
- **Negative**: everything else, including same fold but different
  superfamily.

That last choice is deliberate and it is conservative against us. Domains
sharing a fold but not a superfamily are structurally similar with uncertain
homology, and some published benchmarks exclude them rather than count them
against a method. Counting them as negatives can only lower our numbers, so we
take the stricter reading. Section 7 reports the sensitivity of the headline
number to this choice.

## 3. Query set and candidate pool

**Query set**: every domain in the filtered corpus that has at least one
positive, that is, every domain whose superfamily contains at least one other
domain from a different family. Domains with no possible positive are dropped,
because recall is undefined for them and including them would just divide
every method's score by the same arbitrary constant.

Selection is deterministic and total. There is no sampling. If cost later
forces sampling, it happens with a fixed seed, the seed and N are recorded
here, and the change is dated in the changelog.

**Candidate pool**: for each query, every other domain in the corpus except
the query itself and its same-family domains, per section 2. Identical pool
for every method. A comparison where methods see different candidates is
meaningless.

## 4. Metrics

Let R be the set of positives for a query and `top_k` the method's k
highest-ranked candidates.

**recall@k**, for k in {1, 10, 100}:

    recall@k = |R ∩ top_k| / |R|

The denominator is the total number of positives, not `min(k, |R|)`. When
|R| > k this metric cannot reach 1.0, and that is correct and intended: it is
the same ceiling for every method. Normalising by `min(k, |R|)` would hide how
much a method missed and is the more flattering choice, which is why we are
not using it.

**Mean average precision**: average precision computed per query over the full
ranked list, then averaged across queries. Unlike recall@k this rewards
ranking positives highly rather than merely retrieving them.

**ROC AUC** for the same-superfamily decision, computed per query over the
full candidate pool using each method's score as the discriminant, then
averaged across queries.

**Latency**: wall-clock seconds per query, reported as median and p95, never
as a mean. Reported separately for index search and end-to-end query, per the
definition of done in CLAUDE.md section 10.

### Pre-registered headline

The headline number is **recall@10**, chosen now, before any results exist. If
another metric later tells a more flattering story, that metric is reported
alongside, never in place of this one.

## 5. Methods

All three run over the identical corpus, query set, and candidate pool.

| Method | Configuration |
|---|---|
| BLAST+ `blastp` | Default scoring matrix. E-value cutoff recorded in the results manifest. |
| MMseqs2 `search` | Run twice: at default sensitivity, and at `-s 7.5`. Both reported. |
| esm2-protein-search | Via the HTTP API in `contracts/openapi.yaml`. Index parameters and model checkpoint recorded in the manifest. |

MMseqs2 is run at high sensitivity as well as default because reporting only
its default settings would be the easy way to beat it, and CLAUDE.md section 1
already commits us to treating it as the baseline rather than the competitor.

### Handling short or empty result lists

A method that returns fewer than k results scores misses for the empty
positions. They are not excluded from the denominator.

This matters most for BLAST, which returns nothing below its E-value
threshold. Remote homologs are exactly the cases BLAST declines to report, so
treating a non-answer as anything other than a miss would flatter it
substantially.

### Tie-breaking

Equal scores are broken by candidate identifier, ascending. Deterministic, so
no method gains or loses on ordering luck, and reruns are reproducible.

## 6. Reporting commitments

Made before results exist:

- If MMseqs2 beats us on the headline metric, that is the reported result.
  CLAUDE.md section 1 states an honest negative result is a better artifact
  than a fudged win, and this protocol is where that commitment becomes
  binding rather than aspirational.
- Every method's numbers are reported, including configurations where we
  lose.
- Per-method compute cost is reported: index build time, query latency, peak
  memory.
- The hardware, every tool version, and the SCOPe release accompany every
  results table.

## 7. Sensitivity analyses

Run regardless of whether the headline is favourable:

1. Does the ranking of methods hold across sequence-length bins?
2. Does it hold across superfamily sizes? Large superfamilies have more
   positives and are easier; a method that wins only on those has not shown
   what we would be claiming.
3. Does the headline change if same-fold-different-superfamily pairs are
   excluded rather than counted as negatives, per section 2?

## 8. Known limitations

Written now, not after the fact, and carried into `docs/benchmark.md` in D7.

- SCOPe domains are structural fragments, not full-length proteins. Our
  deployed service indexes whole Swiss-Prot entries. This benchmark measures
  the method, not the deployed artifact; the Pfam-clan secondary check exists
  to partly cover that gap and is reported separately.
- ESM-2 was pretrained on UniRef, which overlaps the sequences these SCOPe
  domains were drawn from. This is not leakage in the supervised sense, as no
  superfamily labels were seen, but it is not a clean holdout either and the
  report says so.
- The 40 percent identity filter is one choice among several defensible ones.
- Truncation at 1022 residues affects long domains. R13 requires the affected
  fraction and its recall be reported separately.

## Changelog

| Date | Change | Results existed? |
|---|---|---|
| 2026-08-24 | Protocol fixed before any implementation or results | No |
