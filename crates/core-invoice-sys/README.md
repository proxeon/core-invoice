# core-invoice-sys

C ABI for `core_invoice_validate` / `_ubl`, convert, diff, version. Returns **0** valid, **1** invalid, **2** unreadable / bad args.

```c
#include "core_invoice.h"
#include <stdio.h>

int main(void) {
    const char *xml = "<!-- load a Peppol UBL document -->";
    char err[512];
    int rc = core_invoice_validate_ubl(xml, "peppol", err, sizeof err);
    if (rc == 0) puts("valid");
    else fprintf(stderr, "%s\n", err);
    return rc;
}
```

Python: [`python/core_invoice.py`](../../python/core_invoice.py) `validate_xml(xml, profile=None) -> 0|1|2` via ctypes on this ABI (build `-p core-invoice-sys` first). Not a second parser. PyPI wheel is Later.

The semantic crate (`core-invoice`) is the WASM target — `cargo build -p core-invoice --target wasm32-unknown-unknown` — the model, not a browser AP. Formats/sys are not that job.
