import json
import os
import sys
from collections import defaultdict

def generate_contention_table(bench_group):
    # Path relative to workspace root
    base_path = os.path.join("target", "criterion", bench_group)
    
    if not os.path.exists(base_path):
        print(f"Error: Directory '{base_path}' not found.")
        print("Make sure you ran `cargo bench` first!")
        return

    # data[crate_name][pool_size] = point_estimate_in_ns
    data = defaultdict(dict)
    crates = set()
    pool_sizes = set()

    for crate in os.listdir(base_path):
        crate_path = os.path.join(base_path, crate)
        
        # Skip files or the criterion 'report' folder
        if not os.path.isdir(crate_path) or crate == "report":
            continue
            
        for size_str in os.listdir(crate_path):
            size_path = os.path.join(crate_path, size_str)
            if not os.path.isdir(size_path):
                continue
                
            try:
                pool_size = int(size_str)
                estimates_path = os.path.join(size_path, "new", "estimates.json")
                
                if os.path.exists(estimates_path):
                    with open(estimates_path, 'r') as f:
                        est = json.load(f)
                        # point_estimate is automatically calculated by Criterion as ns/iter
                        nanos_per_iter = est["mean"]["point_estimate"]
                        
                        data[crate][pool_size] = nanos_per_iter
                        crates.add(crate)
                        pool_sizes.add(pool_size)
            except ValueError:
                # size_str wasn't an integer, skip
                continue
            except Exception as e:
                print(f"Error reading {estimates_path}: {e}")
                continue

    if not crates:
        print("No benchmark data found in the directory.")
        return

    # Sort sizes descending so columns appear as: 1024 (Light), 16 (Medium), 1 (Heavy)
    sorted_sizes = sorted(list(pool_sizes), reverse=True)
    sorted_crates = sorted(list(crates))

    def get_size_label(s):
        if s == 1024: return "Light (1024)"
        if s == 16: return "Medium (16)"
        if s == 1: return "Heavy (1)"
        return f"Size ({s})"

    # Criterion sanitizes folder names (e.g., replaces < > with _ on Windows/Linux)
    # This cleans up the display name for the markdown table
    def clean_crate_name(name):
        return name.replace("Arc_str_", "Arc<str>")

    # Calculate padding dynamically based on the longest crate name
    max_crate_len = max([len(clean_crate_name(c)) for c in sorted_crates] + [18])
    cell_width = 14

    print(f"### {bench_group} [ns/iter]")
    
    # --- Header Row ---
    header_cols = [f"{'Type':<{max_crate_len}}"] + [get_size_label(s).center(cell_width) for s in sorted_sizes]
    print(" | ".join(header_cols))
    
    # --- Markdown Separator Row ---
    sep_cols = [f"{':---':<{max_crate_len}}"] + [f"{':---:':^{cell_width}}" for _ in sorted_sizes]
    print(" | ".join(sep_cols))

    # --- Data Rows ---
    for crate in sorted_crates:
        display_name = clean_crate_name(crate)
        row_cells = [f"{display_name:<{max_crate_len}}"]
        
        for s in sorted_sizes:
            val = data[crate].get(s)
            if val is not None:
                # Format to 2 decimal places
                row_cells.append(f"{val:{cell_width}.2f}")
            else:
                row_cells.append(f"{'-':^{cell_width}}")
                
        print(" | ".join(row_cells))
    print()

if __name__ == "__main__":
    # If using the Criterion name from the previous rust code:
    # python report_contention.py "Clone Drop Contention"
    group = sys.argv[1] if len(sys.argv) > 1 else "Clone Drop Contention"
    generate_contention_table(group)