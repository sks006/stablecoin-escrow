// Step 1: Declare the processor module which handles top-level routing of instruction byte streams.
pub mod processor;
// Step 2: Declare the initialize module which implements the logic to initialize new escrow instances.
pub mod initialize;
// Step 3: Declare the execute_transfer module containing settlement logic and fee distribution.
pub mod execute_transfer;
// Step 4: Declare the cancel_escrow module containing cancellation logic to drain and close accounts.
pub mod cancel_escrow;