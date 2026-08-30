# core-invoice-formats

UBL 2.1 write/read is a **subset scrape**, not a lossless EN 16931 codec.

**CII D16B is not implemented.** `convert --to cii` and CII parse are refused. Do not wrap UBL in a `CrossIndustryInvoice` costume.
