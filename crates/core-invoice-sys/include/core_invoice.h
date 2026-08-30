#ifndef CORE_INVOICE_H
#define CORE_INVOICE_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* 0 valid, 1 invalid, 2 unreadable / bad args.
   profile NULL = auto from BT-24. Known: en16931 | peppol | pint | pint-my.
   err is UTF-8, NUL-terminated if err_len > 0. Truncation is byte-safe. */
int core_invoice_validate_ubl(const char *xml, const char *profile, char *err, size_t err_len);

#ifdef __cplusplus
}
#endif

#endif
