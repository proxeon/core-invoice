# core-invoice-sys

C ABI for `core_invoice_validate_ubl`. Returns **0** valid, **1** invalid, **2** unreadable / bad args.

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

Python bindings are not implemented in 0.1.x. The semantic crate (`core-invoice`) is the WASM target — `cargo build -p core-invoice --target wasm32-unknown-unknown` — not the codecs.
