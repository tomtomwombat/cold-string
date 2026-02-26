import json
import os
import sys
from collections import defaultdict

def generate_tables(bench_group):
    # Path relative to workspace root
    base_path = os.path.join("target", "criterion", bench_group)
    
    if not os.path.exists(base_path):
        print(f"Error: Directory {base_path} not found.")
        return

    data = defaultdict(dict)
    all_ranges = set()
    crates = set()

    if not os.path.exists(base_path):
        return

    for folder in os.listdir(base_path):
        folder_path = os.path.join(base_path, folder)
        if not os.path.isdir(folder_path) or "-len=" not in folder:
            continue
            
        try:
            # Splits at the last occurrence of -len= to handle crate names with hyphens
            crate_part, range_part = folder.rsplit("-len=", 1)
            min_len, max_len = map(int, range_part.split("-"))
            
            estimates_path = os.path.join(folder_path, "new", "estimates.json")
            if os.path.exists(estimates_path):
                with open(estimates_path, 'r') as f:
                    est = json.load(f)
                    # Dividing by 1000 because of the 1000 strings in the bench loop
                    nanos_per_op = est["mean"]["point_estimate"] / 1000.0
                    
                    data[crate_part][(min_len, max_len)] = nanos_per_op
                    all_ranges.add((min_len, max_len))
                    crates.add(crate_part)
        except Exception as e:
            continue

    sorted_crates = sorted(list(crates))
    sorted_ranges = sorted(list(all_ranges), key=lambda x: (x[1], x[0]))

    def print_markdown_table(title, filter_func, label_fmt):
        filtered_ranges = [r for r in sorted_ranges if filter_func(r)]
        if not filtered_ranges:
            return

        print(f"### {title}")
        
        # Header Row
        header_cols = [f"{'Crate':<18}"] + [label_fmt(r).center(10) for r in filtered_ranges]
        print(" | ".join(header_cols))
        
        # Markdown Separator Row
        sep_cols = [f"{':---':<18}"] + [f"{':---:':^10}" for _ in filtered_ranges]
        print(" | ".join(sep_cols))

        # Data Rows
        for crate in sorted_crates:
            row_cells = [f"{crate:<18}"]
            for r in filtered_ranges:
                val = data[crate].get(r)
                row_cells.append(f"{val:10.1f}" if val is not None else f"{'-':^10}")
            print(" | ".join(row_cells))
        print()

    # Table 1: Variable lengths (0..=N)
    print_markdown_table(
        f"{bench_group.capitalize()}: Variable Length (0..=N) [ns/op]",
        lambda r: r[0] == 0,
        lambda r: f"0..={r[1]}"
    )

    # Table 2: Fixed lengths (N..=N)
    print_markdown_table(
        f"{bench_group.capitalize()}: Fixed Length (N..=N) [ns/op]",
        lambda r: r[0] == r[1] and r[0] != 0,
        lambda r: f"{r[1]}..={r[1]}"
    )

if __name__ == "__main__":
    # Usage:
    # python bench/report.py construction
    # python bench/report.py as_str
    group = sys.argv[1] if len(sys.argv) > 1 else "construction"
    generate_tables(group)