"""Summarizes repeated runs in results/final/ into a markdown table.

    python3 evals/report.py

Result files are named `<suite>.<model>.<prompt>.run<N>.jsonl`.
"""

import collections
import json
import pathlib
import statistics

import score

FINAL = pathlib.Path(__file__).parent / "results" / "final"


def main():
    runs = collections.defaultdict(list)
    categories = collections.defaultdict(lambda: collections.defaultdict(list))
    for path in sorted(FINAL.glob("*.jsonl")):
        suite, rest = path.stem.split(".", 1)
        rest, _run = rest.rsplit(".", 1)
        model, prompt = rest.rsplit(".", 1)
        results = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
        passed = [score.score(r)[0] for r in results]
        runs[(model, prompt, suite)].append(100 * sum(passed) / len(passed))
        for result, ok in zip(results, passed):
            categories[(model, prompt, suite)][score.CASES[result["id"]]["category"]].append(ok)

    print("| Model | Prompt | Suite | Mean | Runs |")
    print("|---|---|---|---|---|")
    for key in sorted(runs):
        values = runs[key]
        print(f"| {key[0]} | {key[1]} | {key[2]} | {statistics.mean(values):.1f}% | "
              f"{', '.join(f'{v:.1f}' for v in values)} |")

    print("\n| Model | Prompt | Held-out category accuracy |")
    print("|---|---|---|")
    for key in sorted(categories):
        if key[2] == "overseer_heldout":
            cells = ", ".join(f"{c} {100 * sum(v) / len(v):.0f}%" for c, v in sorted(categories[key].items()))
            print(f"| {key[0]} | {key[1]} | {cells} |")


if __name__ == "__main__":
    main()
