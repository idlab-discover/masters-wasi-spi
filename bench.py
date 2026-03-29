import sys
import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
import seaborn as sns

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
# 2. Data Processing
# ==========================================
# Throughput calculation (Bytes * 8 = bits).
# Dividing bits by microseconds (AvgRTT_us) gives Mbps exactly!
df["Throughput_Mbps"] = (df["Size_Bytes"] * 8) / df["AvgRTT_us"]

# Split data for comparative operations (Heatmap & Bar chart)
df_n = df[df["Environment"] == "Native"].copy()
df_w = df[df["Environment"] == "WASM"].copy()

sns.set_theme(style="whitegrid")
fig = plt.figure(figsize=(18, 12))

# ==========================================
# Plot 1: Convergence Line Chart (Latency vs Size)
# ==========================================
ax1 = plt.subplot(2, 2, 1)
df_20m = df[df["BaudRate"] == 20000000]
sns.lineplot(
    data=df_20m, x="Size_Bytes", y="AvgRTT_us", hue="Environment", marker="o", ax=ax1
)
ax1.set_xscale("log", base=2)
ax1.set_yscale("log")
ax1.set_title("SPI Latency Convergence at 20 MHz", fontsize=14, fontweight="bold")
ax1.set_xlabel("Payload Size (Bytes, Log Scale)")
ax1.set_ylabel("Average RTT (µs, Log Scale)")
ax1.set_xticks(df_n["Size_Bytes"].unique())
ax1.set_xticklabels(df_n["Size_Bytes"].unique())

# ==========================================
# Plot 2: Efficiency Curve (Throughput vs Size)
# ==========================================
ax2 = plt.subplot(2, 2, 2)
sns.lineplot(
    data=df_20m,
    x="Size_Bytes",
    y="Throughput_Mbps",
    hue="Environment",
    marker="s",
    ax=ax2,
)
ax2.set_xscale("log", base=2)
ax2.set_title(
    "Effective Throughput at 20 MHz Bus Speed", fontsize=14, fontweight="bold"
)
ax2.set_xlabel("Payload Size (Bytes, Log Scale)")
ax2.set_ylabel("Effective Throughput (Mbps)")
ax2.axhline(
    20, color="red", linestyle="--", alpha=0.5, label="Theoretical Max (20 Mbps)"
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
heatmap_data.index = [f"{b / 1000000:g} MHz" for b in heatmap_data.index]

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
# Plot 4: Stacked Bar Chart (Constant vs Variable Time for 1 Byte)
# ==========================================
ax4 = plt.subplot(2, 2, 4)
df_1b = merged[merged["Size_Bytes"] == 1].copy()

# Hardware transmission time for 1 byte (8 bits)
df_1b["HW_Time_us"] = (8 / df_1b["BaudRate"]) * 1_000_000

# Software overhead = Total Time - Hardware Time
df_1b["Native_SW_Overhead"] = df_1b["AvgRTT_us_native"] - df_1b["HW_Time_us"]
df_1b["WASM_SW_Overhead"] = df_1b["AvgRTT_us_wasm"] - df_1b["HW_Time_us"]

labels = [f"{b / 1000000:g} MHz" for b in df_1b["BaudRate"]]
x = np.arange(len(labels))
width = 0.35

ax4.bar(
    x - width / 2,
    df_1b["HW_Time_us"],
    width,
    label="Hardware TX Time",
    color="lightblue",
)
ax4.bar(
    x - width / 2,
    df_1b["Native_SW_Overhead"],
    width,
    bottom=df_1b["HW_Time_us"],
    label="Native Software Overhead",
    color="blue",
)

ax4.bar(x + width / 2, df_1b["HW_Time_us"], width, color="lightblue")
ax4.bar(
    x + width / 2,
    df_1b["WASM_SW_Overhead"],
    width,
    bottom=df_1b["HW_Time_us"],
    label="WASM Software Overhead",
    color="orange",
)

ax4.set_title("Time Breakdown for 1-Byte Transfers", fontsize=14, fontweight="bold")
ax4.set_xticks(x)
ax4.set_xticklabels(labels, rotation=45, ha="right")
ax4.set_ylabel("Time (µs)")
ax4.legend()

plt.tight_layout()
plt.show()
