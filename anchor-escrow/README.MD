# 🏦 Anchor Stablecoin Escrow

This directory contains the standard framework-based implementation of the Just-In-Time (JIT) funded Stablecoin Escrow program using the **Anchor** framework.

This implementation serves as the control baseline for our Compute Unit (CU) optimization benchmarks, contrasting standard framework convenience against the zero-allocation bare-metal Native SBF implementation.

---

## 📂 Project Structure

- **`programs/anchor-escrow`**: The main Anchor program codebase containing instruction handlers, state definitions, and custom error types.
- **`Anchor.toml`**: The Anchor configuration file defining workspaces, local validator environments, and deployment settings.

---

## ⚙️ Standard Framework Design

The Anchor implementation handles runtime execution using Anchor's standard abstractions:
1. **Borsh Serialization/Deserialization**: Every transaction dynamically deserializes account state on the heap using Borsh.
2. **Account Wrappers**: Uses Anchor's `Account<'info, T>` wrappers to automatically load and validate signer authorities, ownership rules, and key mappings.
3. **Automatic System Initialization**: Utilizes `init` constraints to automatically create and pay rent for state accounts via CPI.

---

## 📊 Performance Benchmark Baseline

Because of these framework layers, the Anchor implementation has a predictable CPU and heap memory footprint:

- **`ExecuteTransfer` (Settlement)**: **18,183 Compute Units (CUs)**.
- **Overhead**: Borsh serialization accounts for ~4,200 CUs, heap allocations cost ~1,850 CUs, and framework routing/stack frames account for ~2,500 CUs.

### Benchmark Diagram
![Anchor Escrow Performance Profile](../diagrams/anchor_performance.png)

---

## 🚀 Running the Program

To build the Anchor program:
```bash
anchor build
```

To run the Anchor tests:
```bash
anchor test
```
