#!/usr/bin/env python3
"""Analyse a live incremental A/B run and emit the ship/no-ship evidence.

Usage: analyze_live_ab.py <results.json>

Reports the two things that decide it:
  1. Accuracy — is live-incremental worse than batch, and by how much, with a
     paired sign test so a mean shift from one outlier is not mistaken for a
     real regression.
  2. Latency — what the user actually waits after pressing stop.

Also surfaces per-take failures (any take where live is >1 point worse) and the
word-level diffs on the worst offenders, because a mean hides a dropped window.
"""
import json
import re
import sys
import difflib
from statistics import median


def norm(s):
    s = s.lower().replace("%", " percent ").replace("$", " dollars ")
    return re.sub(r"[^a-z0-9' ]", " ", s).split()


def main():
    if len(sys.argv) < 2:
        sys.exit("usage: analyze_live_ab.py <results.json>")
    rows = json.load(open(sys.argv[1]))
    if not rows:
        sys.exit("no takes recorded")

    n = len(rows)
    bw = [r["batch_wer"] for r in rows]
    iw = [r["incremental_wer"] for r in rows]
    dis = [r["disagreement"] for r in rows]
    deltas = [i - b for i, b in zip(iw, bw)]
    live_windows = sum(r.get("worker_committed_during_capture", 0) for r in rows)

    print("=" * 62)
    print(f"LIVE INCREMENTAL A/B — {n} takes")
    print("=" * 62)
    print(f"total audio            : {sum(r['secs'] for r in rows)/60:.1f} min")
    print(f"windows committed LIVE : {live_windows}  (proves the worker raced capture)")
    print()
    print("ACCURACY")
    print(f"  batch WER       mean {sum(bw)/n:6.2f}%   median {median(bw):6.2f}%")
    print(f"  live-incr WER   mean {sum(iw)/n:6.2f}%   median {median(iw):6.2f}%")
    print(f"  delta           mean {sum(deltas)/n:+6.2f}   median {median(deltas):+6.2f} points")
    print(f"  disagreement    mean {sum(dis)/n:6.2f}%")

    worse = [i for i, d in enumerate(deltas) if d > 1.0]
    better = [i for i, d in enumerate(deltas) if d < -1.0]
    same = n - len(worse) - len(better)
    print(f"  takes >1pt worse: {len(worse)}   >1pt better: {len(better)}   within 1pt: {same}")

    # Paired sign test: how likely is this split under "no real difference"?
    nz = [d for d in deltas if abs(d) > 1e-9]
    if nz:
        pos = sum(1 for d in nz if d > 0)
        k, m = min(pos, len(nz) - pos), len(nz)
        # two-sided exact binomial at p=0.5
        from math import comb
        p = sum(comb(m, j) for j in range(0, k + 1)) * 2 / (2 ** m)
        print(f"  sign test       : {pos}/{m} takes worse, two-sided p = {min(1.0, p):.3f}")

    print()
    print("LATENCY (what the user waits after stop)")
    bms = [r.get("batch_ms", 0) for r in rows]
    tms = [r.get("live_tail_ms", 0) for r in rows]
    print(f"  batch decode    mean {sum(bms)/n:7.0f} ms   median {median(bms):7.0f} ms")
    print(f"  live tail only  mean {sum(tms)/n:7.0f} ms   median {median(tms):7.0f} ms")
    if sum(bms):
        print(f"  saved           {100*(sum(bms)-sum(tms))/sum(bms):.1f}%")

    if worse:
        print()
        print("WORST TAKES (live worse than batch by >1 point)")
        for i in sorted(worse, key=lambda i: -deltas[i])[:5]:
            r = rows[i]
            print(f"\n  take {r['take']} ({r['secs']:.0f}s): batch {r['batch_wer']:.2f}% -> live {r['incremental_wer']:.2f}%  ({deltas[i]:+.2f})")
            b, l = norm(r["batch"]), norm(r["live_incremental"])
            shown = 0
            for op, a1, a2, b1, b2 in difflib.SequenceMatcher(None, b, l).get_opcodes():
                if op == "equal" or shown >= 3:
                    continue
                print(f"    batch: ...{' '.join(b[max(0,a1-4):a2+4])}...")
                print(f"    live : ...{' '.join(l[max(0,b1-4):b2+4])}...")
                shown += 1

    print()
    print("=" * 62)
    verdict_ok = (sum(deltas) / n) <= 0.5 and len(worse) <= max(1, n // 10)
    print("ACCURACY GATE :", "PASS" if verdict_ok else "FAIL")
    print("  (pass = mean delta <= +0.5 points and <=10% of takes >1pt worse)")


if __name__ == "__main__":
    main()
