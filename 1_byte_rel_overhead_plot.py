import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
from scipy.optimize import curve_fit
import matplotlib.ticker as ticker
import sys


def generate_ratio_plot(csv_path):
    # Load the data
    try:
        df = pd.read_csv(csv_path)
    except FileNotFoundError:
        print(f"Error: Could not find the file '{csv_path}'")
        return

    # Filter for 1-byte payloads
    df_filtered = df[df["Size_Bytes"] == 1].copy()

    # Separate Native and WASM data
    native_data = df_filtered[df_filtered["Environment"] == "Native"][
        ["BaudRate", "AvgRTT_us"]
    ].rename(columns={"AvgRTT_us": "Native_RTT"})
    wasm_data = df_filtered[df_filtered["Environment"] == "WASM"][
        ["BaudRate", "AvgRTT_us"]
    ].rename(columns={"AvgRTT_us": "WASM_RTT"})

    # Merge on BaudRate to calculate the ratio
    merged = pd.merge(native_data, wasm_data, on="BaudRate").sort_values("BaudRate")
    merged["Ratio"] = merged["WASM_RTT"] / merged["Native_RTT"]

    x_data = merged["BaudRate"].values
    y_data = merged["Ratio"].values

    # Define the mathematical model
    def ratio_func(x, a, b):
        return (a * x + b) / (x + b)

    # Perform the curve fit
    # Initial guesses: a=3.25 (max ratio), b=2.6e6 (inflection point)
    popt, _ = curve_fit(ratio_func, x_data, y_data, p0=[3.25, 2.6e6])
    a_val = popt[0]
    b_val = popt[1]

    # Calculate R-squared
    y_fit = ratio_func(x_data, *popt)
    ss_res = np.sum((y_data - y_fit) ** 2)
    ss_tot = np.sum((y_data - np.mean(y_data)) ** 2)
    r_squared = 1 - (ss_res / ss_tot)
    print(r_squared)

    # Print the parameters to the console
    print("Mathematical Fit Results:")
    print(f"Max Ratio Limit (a): {a_val:.3f}x")
    print(f"Transmission Inflection Point (b): {b_val:.2f}")
    print(f"R-squared: {r_squared:.5f}")

    # Create the plot
    plt.figure(figsize=(12, 7))

    # Plot the raw measured data
    plt.scatter(x_data, y_data, color="orange", alpha=0.5, s=20, label="Measured Data")

    # Plot the fitted curve
    x_curve = np.linspace(0, max(x_data), 500)
    y_curve = ratio_func(x_curve, a_val, b_val)
    plt.plot(
        x_curve,
        y_curve,
        color="blue",
        linewidth=2,
        label=f"Fitted Model (R² = {r_squared:.3f})",
    )

    # Plot the maximum overhead limit line
    plt.axhline(
        y=a_val,
        color="red",
        linestyle="--",
        linewidth=1.5,
        label=f"Max Ratio Limit ({a_val:.2f}x)",
    )

    # Format the axes
    ax = plt.gca()

    # Format X-axis to show MHz or kHz
    def format_baud(x, pos):
        if x >= 1e6:
            return f"{x * 1e-6:.1f} MHz".replace(".0", "")
        elif x >= 1e3:
            return f"{int(x * 1e-3)} kHz"
        return f"{int(x)}"

    ax.xaxis.set_major_formatter(ticker.FuncFormatter(format_baud))

    # Format Y-axis to show the multiplier 'x'
    ax.yaxis.set_major_formatter(
        ticker.FuncFormatter(lambda y, pos: f"{y:.1f}x".replace(".0x", "x"))
    )

    # Set labels and grid
    plt.xlabel("Baud Rate", fontsize=11)
    plt.ylabel("Execution Time Ratio", fontsize=11)
    plt.ylim(0, 4.0)
    plt.xlim(0, max(x_data))

    plt.grid(True, linestyle=":", alpha=0.7)
    plt.legend(loc="lower right", fontsize=10)

    plt.tight_layout()

    # Display the graph in a window
    plt.show()


if __name__ == "__main__":
    # Take CSV path from command line arguments, or default to "linux_data3.csv"
    input_csv = sys.argv[1] if len(sys.argv) > 1 else "linux_data3.csv"
    print(f"Analyzing data from: {input_csv}\n")

    generate_ratio_plot(input_csv)
