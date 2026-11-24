//! Data-flow analysis support.
//!
//! The initial forward worklist is implemented in [`crate::nullcheck`]. A
//! generic lattice/transfer abstraction will be extracted once a second
//! analysis needs it; premature abstraction would obscure the nullness-specific
//! then/else store handling.
