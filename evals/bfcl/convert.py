"""Converts BFCL v4 data (raw/) into harness cases (../cases/bfcl_*.jsonl).

BFCL (Berkeley Function Calling Leaderboard) is Apache-2.0 licensed:
https://github.com/ShishirPatil/gorilla/tree/main/berkeley-function-call-leaderboard
"""

import json
import pathlib

HERE = pathlib.Path(__file__).parent
TYPE_MAP = {"dict": "object", "float": "number", "tuple": "array", "any": "string"}


def schema(node):
    if isinstance(node, dict):
        node = {k: schema(v) for k, v in node.items()}
        if isinstance(node.get("type"), str) and node["type"] in TYPE_MAP:
            node["type"] = TYPE_MAP[node["type"]]
        return node
    if isinstance(node, list):
        return [schema(v) for v in node]
    return node


def tool_name(name):
    return name.replace(".", "_")


def lines(path):
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def convert(suite, data_file, answer_file):
    answers = {a["id"]: a["ground_truth"] for a in lines(answer_file)} if answer_file else {}
    out = HERE.parent / "cases" / f"bfcl_{suite}.jsonl"
    with out.open("w") as f:
        for row in lines(data_file):
            tools = [
                {"name": tool_name(fn["name"]), "description": fn["description"],
                 "parameters": schema(fn["parameters"])}
                for fn in row["function"]
            ]
            expected = [{tool_name(name): args for call in answers[row["id"]] for name, args in call.items()}] \
                if row["id"] in answers else None
            case = {"id": row["id"], "suite": f"bfcl_{suite}", "category": suite, "system": "none",
                    "prompt": row["question"][0][-1]["content"], "tools": tools, "bfcl_expected": expected}
            f.write(json.dumps(case) + "\n")
    print(f"{out.name}: {len(lines(data_file))} cases")


convert("simple", HERE / "raw/BFCL_v4_simple_python.json", HERE / "raw/BFCL_v4_simple_python.answer.json")
convert("multiple", HERE / "raw/BFCL_v4_multiple.json", HERE / "raw/BFCL_v4_multiple.answer.json")
convert("irrelevance", HERE / "raw/BFCL_v4_irrelevance.json", None)
