from collections import defaultdict
import json
from pathlib import Path
import re

# Mapping from benchmark operation to inner loop iteration count
# (Operations not listed default to 1)
BENCH_COUNTS = {
    "len": 1,
    "eq": 1000,
    "new": 1000,
    "as_str": 1000,
}

# Mapping from folder/sanitized names to clean Rust type names
NAME_MAP = {
    "String": "String",
    "cold_string::ColdString": "cold_string::ColdString",
    "cold_string__coldstring": "cold_string::ColdString",
    "compact_string::CompactString": "compact_string::CompactString",
    "compact_string__compactstring": "compact_string::CompactString",
    "compact_str::CompactString": "compact_str::CompactString",
    "compact_str__compactstring": "compact_str::CompactString",
    "smartstring::alias::String": "smartstring::alias::String",
    "smartstring__alias__string": "smartstring::alias::String",
    "smallstr::smallstring_[u8; 8]": "smallstr::SmallString<[u8; 8]>",
    "smallstr__smallstring_[u8; 8]": "smallstr::SmallString<[u8; 8]>",
    "smallstr__smallstring__u8__8_": "smallstr::SmallString<[u8; 8]>",
    "smol_str::SmolStr": "smol_str::SmolStr",
    "smol_str__smolstr": "smol_str::SmolStr",
}


def clean_type_name(raw_name: str) -> str:
    """Resolves clean type name using NAME_MAP with fallback string cleanup."""
    if raw_name in NAME_MAP:
        return NAME_MAP[raw_name]

    lowered = raw_name.lower()
    for key, mapped_val in NAME_MAP.items():
        if key.lower() == lowered:
            return mapped_val

    return raw_name.replace("__", "::")


def parse_param_sort_key(param_str: str):
    """Extracts numbers for natural sorting (e.g., 'len=0..=4' -> [0.0, 4.0])."""
    numbers = re.findall(r"\d+(?:\.\d+)?", param_str)
    return [float(n) for n in numbers] if numbers else [param_str]


def parse_criterion_results(
    target_dir: str = "target/criterion", metric: str = "mean"
):
    criterion_path = Path(target_dir)
    if not criterion_path.exists():
        print(f"Directory '{target_dir}' not found.")
        return

    # Data structure: data[operation][type_name][param_id] = ns_val
    data = defaultdict(lambda: defaultdict(dict))

    for estimates_file in criterion_path.glob("**/new/estimates.json"):
        benchmark_dir = estimates_file.parent.parent

        group_name = None
        bench_id_str = None
        bench_info_file = benchmark_dir / "benchmark.json"

        if bench_info_file.exists():
            try:
                with open(bench_info_file, "r") as f:
                    info = json.load(f)
                    group_name = (
                        info.get("group_id")
                        or (
                            info.get("full_id", "").split("/")[0]
                            if "/" in info.get("full_id", "")
                            else None
                        )
                        or info.get("directory_name")
                    )

                    if info.get("value_str"):
                        bench_id_str = info["value_str"]
                    elif info.get("function_id"):
                        bench_id_str = info["function_id"]
                    elif info.get("full_id") and "/" in info["full_id"]:
                        bench_id_str = info["full_id"].rsplit("/", 1)[1]
            except Exception:
                pass

        if not group_name:
            group_name = benchmark_dir.parent.name

        # Split group_name into (raw_type, operation)
        if "::" in group_name:
            raw_type, operation = group_name.rsplit("::", 1)
        elif "___" in group_name:
            raw_type, operation = group_name.rsplit("___", 1)
        elif "__" in group_name:
            raw_type, operation = group_name.rsplit("__", 1)
        else:
            raw_type, operation = group_name, "bench"

        type_name = clean_type_name(raw_type)

        if not bench_id_str:
            bench_id_str = benchmark_dir.name

        param_str = bench_id_str.replace(",", ", ").strip()

        try:
            with open(estimates_file, "r") as f:
                estimates = json.load(f)

            point_estimate_ns = estimates.get(metric, {}).get("point_estimate")
            if point_estimate_ns is not None:
                data[operation][type_name][param_str] = point_estimate_ns
        except (json.JSONDecodeError, KeyError, OSError):
            continue

    if not data:
        print(f"No valid Criterion results found in '{target_dir}'.")
        return

    # Render Markdown table for each operation
    for operation in sorted(data.keys()):
        op_data = data[operation]

        # Determine divisor for iterations (defaults to 1 if not explicitly in BENCH_COUNTS)
        count_per_bench = BENCH_COUNTS.get(operation, 1)

        all_params = sorted(
            {p for t in op_data.values() for p in t.keys()},
            key=parse_param_sort_key,
        )
        sorted_types = sorted(op_data.keys())

        headers = ["Type"] + all_params
        rows = []

        for type_name in sorted_types:
            if any(param not in op_data[type_name] for param in all_params):
                continue

            row = [f"`{type_name}`"]
            for param in all_params:
                raw_val = op_data[type_name][param]
                # Scale by batch count to get ns per single operation
                per_op_ns = raw_val / count_per_bench
                row.append(f"{per_op_ns:.2f}")

            rows.append(row)

        if not rows:
            continue

        # Format Markdown Table
        col_widths = [
            max(len(str(cell)) for cell in col) for col in zip(headers, *rows)
        ]

        def format_row(r):
            return (
                "| "
                + " | ".join(
                    f"{str(cell):<{w}}" for cell, w in zip(r, col_widths)
                )
                + " |"
            )

        header_line = format_row(headers)
        separator = "| " + " | ".join("-" * w for w in col_widths) + " |"
        table_lines = [header_line, separator] + [format_row(r) for r in rows]

        print(f"### Benchmark: `{operation}` (ns/op)\n")
        print("\n".join(table_lines))
        print("\n" + "-" * 80 + "\n")


if __name__ == "__main__":
    parse_criterion_results(metric="mean")