import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
import seaborn as sns
import io

# 1. Load the Data
native_data = """BaudRate,Size_Bytes,TotalTime_us,AvgRTT_us,LoopbackValid
100000,1,100291,100.29,true
100000,2,190614,190.61,true
100000,4,370854,370.85,true
100000,8,730891,730.89,true
100000,16,1457581,1457.58,true
100000,32,2924544,2924.54,true
100000,64,5799244,5799.24,true
100000,128,10286700,10286.70,true
100000,256,20545458,20545.46,true
100000,512,41030218,41030.22,true
100000,1024,81999993,81999.99,true
100000,2048,163929835,163929.83,true
100000,4096,327785334,327785.33,true
500000,1,48577,48.58,true
500000,2,55704,55.70,true
500000,4,90886,90.89,true
500000,8,167117,167.12,true
500000,16,318134,318.13,true
500000,32,613492,613.49,true
500000,64,1185514,1185.51,true
500000,128,2103520,2103.52,true
500000,256,4153071,4153.07,true
500000,512,8250744,8250.74,true
500000,1024,16446919,16446.92,true
500000,2048,32844785,32844.79,true
500000,4096,65618367,65618.37,true
1000000,1,16146,16.15,true
1000000,2,25191,25.19,true
1000000,4,68359,68.36,true
1000000,8,84618,84.62,true
1000000,16,157631,157.63,true
1000000,32,304253,304.25,true
1000000,64,589843,589.84,true
1000000,128,1043976,1043.98,true
1000000,256,2068316,2068.32,true
1000000,512,4146996,4147.00,true
1000000,1024,8252326,8252.33,true
1000000,2048,16448365,16448.37,true
1000000,4096,32843979,32843.98,true
2000000,1,11795,11.79,true
2000000,2,16430,16.43,true
2000000,4,25326,25.33,true
2000000,8,64578,64.58,true
2000000,16,85018,85.02,true
2000000,32,159656,159.66,true
2000000,64,301130,301.13,true
2000000,128,531467,531.47,true
2000000,256,1044048,1044.05,true
2000000,512,2070417,2070.42,true
2000000,1024,4149455,4149.45,true
2000000,2048,8256903,8256.90,true
2000000,4096,16458729,16458.73,true
5000000,1,9236,9.24,true
5000000,2,10092,10.09,true
5000000,4,13771,13.77,true
5000000,8,20963,20.96,true
5000000,16,59159,59.16,true
5000000,32,74225,74.22,true
5000000,64,128192,128.19,true
5000000,128,223923,223.92,true
5000000,256,429124,429.12,true
5000000,512,840142,840.14,true
5000000,1024,1660958,1660.96,true
5000000,2048,3319220,3319.22,true
5000000,4096,6623964,6623.96,true
10000000,1,8503,8.50,true
10000000,2,9329,9.33,true
10000000,4,10491,10.49,true
10000000,8,12339,12.34,true
10000000,16,19546,19.55,true
10000000,32,33900,33.90,true
10000000,64,74464,74.46,true
10000000,128,121412,121.41,true
10000000,256,224228,224.23,true
10000000,512,430703,430.70,true
10000000,1024,840648,840.65,true
10000000,2048,1661275,1661.28,true
10000000,4096,3325087,3325.09,true
15000000,1,8225,8.22,true
15000000,2,8754,8.75,true
15000000,4,10015,10.02,true
15000000,8,12514,12.51,true
15000000,16,17440,17.44,true
15000000,32,25316,25.32,true
15000000,64,51842,51.84,true
15000000,128,89519,89.52,true
15000000,256,159448,159.45,true
15000000,512,299051,299.05,true
15000000,1024,578769,578.77,true
15000000,2048,1137468,1137.47,true
15000000,4096,2254872,2254.87,true
20000000,1,3142,3.14,true
20000000,2,3587,3.59,true
20000000,4,4535,4.54,true
20000000,8,6413,6.41,true
20000000,16,10158,10.16,true
20000000,32,17661,17.66,true
20000000,64,41999,42.00,true
20000000,128,73092,73.09,true
20000000,256,126516,126.52,true
20000000,512,245803,245.80,true
20000000,1024,475511,475.51,true
20000000,2048,905714,905.71,true
20000000,4096,1772505,1772.51,true"""

wasm_data = """BaudRate,Size_Bytes,TotalTime_us,AvgRTT_us,LoopbackValid
100000,1,110094,110.09,true
100000,2,201385,201.38,true
100000,4,381254,381.25,true
100000,8,740730,740.73,true
100000,16,1460781,1460.78,true
100000,32,2940783,2940.78,true
100000,64,5834138,5834.14,true
100000,128,10340768,10340.77,true
100000,256,20581407,20581.41,true
100000,512,41073074,41073.07,true
100000,1024,82039130,82039.13,true
100000,2048,164000024,164000.02,true
100000,4096,327880867,327880.87,true
500000,1,65295,65.30,true
500000,2,73159,73.16,true
500000,4,92982,92.98,true
500000,8,166016,166.02,true
500000,16,311091,311.09,true
500000,32,601719,601.72,true
500000,64,1175979,1175.98,true
500000,128,2079050,2079.05,true
500000,256,4178292,4178.29,true
500000,512,8287763,8287.76,true
500000,1024,16489894,16489.89,true
500000,2048,32895932,32895.93,true
500000,4096,65698722,65698.72,true
1000000,1,35844,35.84,true
1000000,2,36980,36.98,true
1000000,4,72966,72.97,true
1000000,8,93876,93.88,true
1000000,16,166976,166.98,true
1000000,32,313646,313.65,true
1000000,64,599544,599.54,true
1000000,128,1054636,1054.64,true
1000000,256,2079483,2079.48,true
1000000,512,4179215,4179.22,true
1000000,1024,8292104,8292.10,true
1000000,2048,16501972,16501.97,true
1000000,4096,32916112,32916.11,true
2000000,1,33970,33.97,true
2000000,2,38878,38.88,true
2000000,4,42899,42.90,true
2000000,8,58717,58.72,true
2000000,16,95070,95.07,true
2000000,32,169615,169.62,true
2000000,64,311310,311.31,true
2000000,128,542114,542.11,true
2000000,256,1055102,1055.10,true
2000000,512,2080512,2080.51,true
2000000,1024,4184497,4184.50,true
2000000,2048,8307555,8307.56,true
2000000,4096,16525168,16525.17,true
5000000,1,28604,28.60,true
5000000,2,28712,28.71,true
5000000,4,32477,32.48,true
5000000,8,32070,32.07,true
5000000,16,51832,51.83,true
5000000,32,82857,82.86,true
5000000,64,138013,138.01,true
5000000,128,234661,234.66,true
5000000,256,440207,440.21,true
5000000,512,851109,851.11,true
5000000,1024,1672870,1672.87,true
5000000,2048,3356699,3356.70,true
5000000,4096,6691219,6691.22,true
10000000,1,30267,30.27,true
10000000,2,20966,20.97,true
10000000,4,21302,21.30,true
10000000,8,24911,24.91,true
10000000,16,32208,32.21,true
10000000,32,40332,40.33,true
10000000,64,80034,80.03,true
10000000,128,132274,132.27,true
10000000,256,235218,235.22,true
10000000,512,441376,441.38,true
10000000,1024,853440,853.44,true
10000000,2048,1678294,1678.29,true
10000000,4096,3372063,3372.06,true
15000000,1,27086,27.09,true
15000000,2,23188,23.19,true
15000000,4,24427,24.43,true
15000000,8,26925,26.93,true
15000000,16,27432,27.43,true
15000000,32,29502,29.50,true
15000000,64,61255,61.26,true
15000000,128,99492,99.49,true
15000000,256,169741,169.74,true
15000000,512,310220,310.22,true
15000000,1024,591186,591.19,true
15000000,2048,1153924,1153.92,true
15000000,4096,2276805,2276.80,true
20000000,1,10426,10.43,true
20000000,2,10793,10.79,true
20000000,4,11712,11.71,true
20000000,8,13616,13.62,true
20000000,16,17382,17.38,true
20000000,32,24915,24.91,true
20000000,64,51526,51.53,true
20000000,128,82919,82.92,true
20000000,256,156433,156.43,true
20000000,512,273110,273.11,true
20000000,1024,498973,498.97,true
20000000,2048,947420,947.42,true
20000000,4096,1819611,1819.61,true"""

df_n = pd.read_csv(io.StringIO(native_data))
df_n["Environment"] = "Native Linux"

df_w = pd.read_csv(io.StringIO(wasm_data))
df_w["Environment"] = "WASM Linux"

df = pd.concat([df_n, df_w])

# FIXED: Throughput calculation (Bytes * 8 = bits).
# Dividing bits by microseconds (AvgRTT_us) gives Mbps exactly!
df["Throughput_Mbps"] = (df["Size_Bytes"] * 8) / df["AvgRTT_us"]

sns.set_theme(style="whitegrid")
fig = plt.figure(figsize=(18, 12))

# ==========================================
# 1. Convergence Line Chart (Latency vs Size)
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
# 2. Efficiency Curve (Throughput vs Size)
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
# Now the Y-axis will top out around 18.5 Mbps, so the line at 20 will correctly sit at the top.
ax2.axhline(
    20, color="red", linestyle="--", alpha=0.5, label="Theoretical Max (20 Mbps)"
)
ax2.legend()

# ==========================================
# 3. The Overhead Heatmap (Times Slower)
# ==========================================
ax3 = plt.subplot(2, 2, 3)
merged = pd.merge(
    df_n, df_w, on=["BaudRate", "Size_Bytes"], suffixes=("_native", "_wasm")
)

# CHANGED: Calculate "Times Slower" instead of percentage
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
# 4. Stacked Bar Chart (Constant vs Variable Time for 1 Byte)
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
ax4.set_xticklabels(labels, rotation=45, ha="right")  # Tilted labels for readability
ax4.set_ylabel("Time (µs)")
ax4.legend()

plt.tight_layout()
plt.show()
