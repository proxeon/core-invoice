#ifndef CORE_INVOICE_H
#define CORE_INVOICE_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* 0 valid, 1 invalid, 2 unreadable / bad args.
   profile NULL = auto from BT-24. Known: en16931 | peppol | pint | pint-my.
   err is UTF-8, NUL-terminated if err_len > 0. Truncation is byte-safe.
   Opaque validate only — Invoice layout is not frozen. */
int core_invoice_validate_ubl(const char *xml, const char *profile, char *err, size_t err_len);
int core_invoice_validate(const char *xml, const char *profile, char *err, size_t err_len);
int core_invoice_convert(const char *xml, const char *to, const char *profile, char *out, size_t out_len, char *err, size_t err_len);
int core_invoice_diff(const char *left, const char *right, char *out, size_t out_len, char *err, size_t err_len);
int core_invoice_version(char *out, size_t out_len);

#ifdef __cplusplus
}
#endif

#endif
