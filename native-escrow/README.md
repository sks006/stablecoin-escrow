
### 🧱 Phase 1: The Memory Layer (`state.rs`)

This is the physical foundation. Instead of relying on a framework to serialize data, we built a strict C-ABI memory stencil.

* **The Struct (`NativeEscrowState`):** Enforces a rigid 204-byte footprint using `#[repr(C)]`.
* **Zero-Copy Alignment:** We manually calculated the 8-byte CPU boundaries and inserted `_padding: [0; 4]` to prevent undefined behavior and memory corruption.
* **Type Safety:** A cryptographic 8-byte discriminator (`b"escrowv1"`) guarantees we never overlay our stencil onto the wrong account type.

### 🔀 Phase 2: The Dispatcher (`processor.rs`)

This is the front door. The Sealevel Virtual Machine (SVM) delivers a raw stream of network traffic (`&[u8]`).

* **The Router:** We extract the very first byte (`instruction_data[0]`).
* **The Matrix:** * `0` = Route to `initialize.rs`
* `1` = Route to `execute_transfer.rs`
* `2` = Route to `cancel_escrow.rs`


* **The Handoff:** The dispatcher strips the routing byte and safely passes the *remaining* payload (`&instruction_data[1..]`) to the specific handler.

### 🏗️ Phase 3: The Initialization Engine (`initialize.rs`)

This is where the program interacts with physical ledger hardware.

* **The Gatekeeper Paradox:** We check if the account is completely unallocated (`data_len == 0`) before asserting program ownership, preventing immediate runtime crashes on fresh transactions.
* **Hardware Allocation (System CPI):** We execute `invoke_signed`, passing the multi-dimensional seed matrix `&[&[b"escrow", ..., &[escrow_bump]]]`. This proves the PDA's identity and commands the System Program to allocate our 204 bytes.
* **Zero-Copy Overlay:** Using `try_from_bytes_mut`, we break into the raw memory heap and securely stamp the `NativeEscrowState` parameters directly onto the bytes.
* **Capital Lockup (Token CPI):** We execute `transfer_checked` to move the `maker`'s stablecoins into the newly verified PDA vault.

### 🛡️ The Enterprise Vulnerability Matrix

Our design explicitly handles these critical native edge cases:

* **Memory Hazards:** We prevent misaligned pointers (`try_from_bytes_mut`), out-of-bounds panics (exact 106-byte length checks), and data overwrites (discriminator/initialization checks).
* **Identity Spoofing:** We programmatically derive both the state and vault PDAs using `Pubkey::find_program_address` to prevent account substitution attacks.
* **Economic Drift:** We read the live `decimals` from the mint account to ensure Token-2022 transfers do not truncate or miscalculate custom stablecoin architectures.

---

### 🌊 Phase 4: The Settlement Engine (`execute_transfer.rs`)

The escrow is fully funded and waiting on the ledger.

When the neobank triggers `execute_transfer`, the first step is to transfer the stablecoins from the `escrow_vault` to the `taker`. Once that transfer clears, the `escrow_vault` token account will hold exactly 0 tokens.

To maintain perfect capital efficiency for the neobank, we must completely destroy this empty token account and sweep its underlying SOL rent (the "dust") back to the `maker` who originally paid for it.

Because the SPL Token Program physically owns the vault, we cannot wipe its data manually. We must send a Cross-Program Invocation commanding the Token Program to do it.

