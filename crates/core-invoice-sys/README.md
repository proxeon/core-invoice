# core-invoice-sys

C ABI (`include/core_invoice.h`): `core_invoice_validate_ubl`.

Returns **0** valid, **1** invalid, **2** unreadable / unknown profile. `profile` NULL means auto from BT-24.

Python and WASM are **not** implemented yet; they will bind to this crate.
