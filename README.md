# stablecoin-escrow
two stablecoin escrow implementations: one with Anchor (heap-heavy) and one with zero-allocation (native). 


```
/stablecoin-escrow-monorepo
├── Cargo.toml                       # Workspace configuration
├── package.json                     # Root scripts to orchestrate Surfpool
├── surfpool.toml                    # Surfpool configuration for JIT state hydration
├── anchor-escrow/                   # PATHWAY 1: Standard Framework Setup
│   ├── Anchor.toml
│   ├── Cargo.toml
│   └── programs/
│       └── anchor-escrow/
│           ├── Cargo.toml
│           └── src/
│               ├── lib.rs           # Borsh instruction routing & macros
│               ├── errors.rs        # Compliance error codes
│               ├── instruction/   # Anchor instruction handlers
|               |    ├── mod.rs
|               |    ├── initialize.rs
|               |    ├── cancel_escrow.rs
|               |    └── execute_transfer.rs   (placeholder)
│               └── state.rs         # Dynamic heap state layouts
├── native-escrow/                   # PATHWAY 2: Bare-Metal Bare-Slicing Setup
│   ├── Cargo.toml                   # Optimized for SBF target (panic = "abort")
│   ├── src/
│   │   ├── lib.rs                   # C-ABI export & no_mangle registration
│   │   ├── entrypoint.rs            # Raw runtime stream parsing
│   │   ├── state.rs                 # #[repr(C)] + Bytemuck unaligned layouts
│   │   ├── errors.rs                # Compliance error codes
│   │   └── instruction/             
│   │       ├── mod.rs               # Instruction module declaration
│   │       └── processor.rs         # O(1) Zero-allocation evaluation engine
│   └── tests/                       
│       └── litesvm_tests.rs         # 🧪 LITESVM BENCHMARK SUITE (In-process execution)
└── tests/                           # 🏄‍♂️ SURFPOOL INTEGRATION SUITE
    ├── anchor_escrow.ts             # Anchor TS client (CU assertion)
    ├── zero_escrow.ts               # Native TS client (CU assertion)
    └── utils.ts                     # Shared setup: Mainnet EURC hydration hooks

```
