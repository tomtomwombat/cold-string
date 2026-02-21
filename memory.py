import os
import glob
import matplotlib.pyplot as plt
from math import log10
from matplotlib import colormaps

plt.rcParams['font.size'] = 20
cm = [colormaps['Set2'](i / 8) for i in range(8)]

def read_csv(path):
    xs = []
    ys = []
    with open(path, "r") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            x, y = line.split(",")
            xs.append(float(x))
            ys.append(float(y))
    return xs, ys

def main():
    csv_files = glob.glob("*.csv")

    if not csv_files:
        print("No CSV files found in current directory.")
        return

    plt.figure(figsize=(10, 6))

    for file in sorted(csv_files):
        xs, ys = read_csv(file)
        label = os.path.splitext(os.path.basename(file))[0]
        plt.plot(xs, ys, label=label, linewidth=3.5)

    plt.xlabel("String Length")
    plt.ylabel("Memory Usage (bytes)")
    plt.title("String Memory Comparison")
    plt.grid(True, linestyle="--", alpha=0.5)
    plt.legend()
    plt.xlim(left=-1)
    plt.ylim(bottom=-1)
    plt.tight_layout()
    plt.yticks(range(0, 100, 8))
    plt.xticks(range(0, 48, 4))
    plt.show()


if __name__ == "__main__":
    main()
