import sys
import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
import seaborn as sns
from matplotlib.ticker import ScalarFormatter

# ==========================================
# 1. Load Data from Command Line Argument
# ==========================================
if len(sys.argv) < 2:
    print("Usage: python bench.py <results.csv>")
    sys.exit(1)

csv_file = sys.argv[1]

try:
    df = pd.read_csv(csv_file)
except FileNotFoundError:
    print(f"Error: Could not find file '{csv_file}'")
    sys.exit(1)

if "Environment" not in df.columns:
    print("Error: CSV must contain an 'Environment' column.")
    sys.exit(1)

# ==========================================
# 2. Data Processing & Dynamic Environment Setup
# ==========================================
# Throughput calculation (Bytes * 8 = bits).
# Dividing bits by microseconds (AvgRTT_us) gives Mbps exactly!
df["Throughput_Mbps"] = (df["Size_Bytes"] * 8) / df["AvgRTT_us"]

# Dynamically identify the environments (assuming exactly 2 for comparison)
envs = df["Environment"].unique()
if len(envs) < 2:
    print("Error: Need at least 2 environments to compare.")
    sys.exit(1)

# Try to find the "Native" environment dynamically, assign the other to WASM
env_native = next((e for e in envs if "Native" in e), envs[0])
env_wasm = next((e for e in envs if e != env_native), envs[1])

print(f"Comparing: '{env_native}' (Native) vs '{env_wasm}' (WASM)")

df_n = df[df["Environment"] == env_native].copy()
df_w = df[df["Environment"] == env_wasm].copy()

sns.set_theme(style="whitegrid")
# Slightly increased width to comfortably fit unrotated labels on the bar chart
fig = plt.figure(figsize=(20, 12))


# Helper to format Baud Rates cleanly (kHz or MHz)
def format_baud(b):
    if b >= 1_000_000:
        return f"{b / 1_000_000:g} MHz"
    else:
        return f"{b / 1000:g} kHz"


# Find Max Baud Rate dynamically for the first two plots
max_baud = df["BaudRate"].max()
df_max_baud = df[df["BaudRate"] == max_baud]

# Formatter to force standard integers instead of scientific/math notation like 2^3
scalar_formatter = ScalarFormatter()
scalar_formatter.set_scientific(False)

# ==========================================
# Plot 1: Convergence Line Chart (Latency vs Size)
# ==========================================
ax1 = plt.subplot(2, 2, 1)
sns.lineplot(
    data=df_max_baud,
    x="Size_Bytes",
    y="AvgRTT_us",
    hue="Environment",
    marker="o",
    ax=ax1,
)
ax1.set_xscale("log", base=2)
ax1.set_yscale("log", base=10)
ax1.set_title(
    f"SPI Latency Convergence at {format_baud(max_baud)}",
    fontsize=14,
    fontweight="bold",
)
# Explicitly stating log scale bases
ax1.set_xlabel("Payload Size (Bytes, log2 scale)")
ax1.set_ylabel("Average RTT (µs, log10 scale)")

# Forcing full numbers for payload sizes
ax1.xaxis.set_major_formatter(scalar_formatter)
ax1.set_xticks(df_max_baud["Size_Bytes"].unique())

# ==========================================
# Plot 2: Efficiency Curve (Throughput vs Size)
# ==========================================
ax2 = plt.subplot(2, 2, 2)
sns.lineplot(
    data=df_max_baud,
    x="Size_Bytes",
    y="Throughput_Mbps",
    hue="Environment",
    marker="s",
    ax=ax2,
)
ax2.set_xscale("log", base=2)
ax2.set_title(
    f"Effective Throughput at {format_baud(max_baud)} Bus Speed",
    fontsize=14,
    fontweight="bold",
)
# Explicitly stating log scale base
ax2.set_xlabel("Payload Size (Bytes, log2 scale)")
ax2.set_ylabel("Effective Throughput (Mbps)")

# Forcing full numbers for payload sizes
ax2.xaxis.set_major_formatter(scalar_formatter)
ax2.set_xticks(df_max_baud["Size_Bytes"].unique())

# Plot theoretical max throughput based on max baud rate
max_mbps = max_baud / 1_000_000
ax2.axhline(
    max_mbps,
    color="red",
    linestyle="--",
    alpha=0.5,
    label=f"Theoretical Max ({max_mbps:g} Mbps)",
)
ax2.legend()

# ==========================================
# Plot 3: The Overhead Heatmap (Times Slower)
# ==========================================
ax3 = plt.subplot(2, 2, 3)
merged = pd.merge(
    df_n, df_w, on=["BaudRate", "Size_Bytes"], suffixes=("_native", "_wasm")
)

# Calculate "Times Slower"
merged["Overhead_Times"] = merged["AvgRTT_us_wasm"] / merged["AvgRTT_us_native"]

heatmap_data = merged.pivot(
    index="BaudRate", columns="Size_Bytes", values="Overhead_Times"
)
heatmap_data = heatmap_data.sort_index(ascending=False)
heatmap_data.index = [format_baud(b) for b in heatmap_data.index]

sns.heatmap(
    heatmap_data,
    annot=True,
    fmt=".1f",
    cmap="YlOrRd",
    ax=ax3,
    cbar_kws={"label": "Multiplier (x times slower)"},
)
ax3.set_title("WASM Overhead Multiplier (Times Slower)", fontsize=14, fontweight="bold")
ax3.set_xlabel("Payload Size (Bytes)")
ax3.set_ylabel("SPI Bus Baud Rate")

# ==========================================
# Plot 4: Stacked Bar Chart (Hardware at the bottom)
# ==========================================
ax4 = plt.subplot(2, 2, 4)

# Find the smallest payload size dynamically (usually 1 byte)
min_size = merged["Size_Bytes"].min()
df_min = merged[merged["Size_Bytes"] == min_size].copy()

# Hardware transmission time for the minimum bytes (Bits / BaudRate * 1,000,000 µs)
df_min["HW_Time_us"] = ((min_size * 8) / df_min["BaudRate"]) * 1_000_000

# Software overhead = Total Time - Hardware Time
df_min["Native_SW_Overhead"] = df_min["AvgRTT_us_native"] - df_min["HW_Time_us"]
df_min["WASM_SW_Overhead"] = df_min["AvgRTT_us_wasm"] - df_min["HW_Time_us"]

labels = [format_baud(b) for b in df_min["BaudRate"]]
x = np.arange(len(labels))
width = 0.35

# 1. Plot Native bars (Hardware Time on bottom, Overhead on top)
ax4.bar(
    x - width / 2,
    df_min["HW_Time_us"],
    width,
    label="Hardware TX Time",
    color="lightblue",
)
ax4.bar(
    x - width / 2,
    df_min["Native_SW_Overhead"],
    width,
    bottom=df_min["HW_Time_us"],
    label="Native Software Overhead",
    color="blue",
)

# 2. Plot WASM bars (Hardware Time on bottom, Overhead on top)
ax4.bar(
    x + width / 2,
    df_min["HW_Time_us"],
    width,
    color="lightblue",  # No label here so it doesn't duplicate in the legend
)
ax4.bar(
    x + width / 2,
    df_min["WASM_SW_Overhead"],
    width,
    bottom=df_min["HW_Time_us"],
    label="WASM Software Overhead",
    color="orange",
)

ax4.set_title(
    f"Time Breakdown for {min_size}-Byte Transfers", fontsize=14, fontweight="bold"
)
ax4.set_xticks(x)
ax4.set_xticklabels(labels, rotation=0, ha="center")
ax4.set_ylabel("Time (µs)")
ax4.legend()

plt.tight_layout()
plt.show()
