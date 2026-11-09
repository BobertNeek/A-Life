# Qualified Optional-Quiet cache preservation

All26 fixed V2 executions passed, including complete direct byte equality of uncached/cached outputs and cold/owner/rollback controls. Read README.md for scope and RESULTS-CURATED.json for bounded evidence. This is not a measured speedup or a learned-policy improvement. The coordinated cache variant remains unqualified.

Before source reconstruction, read production/transport/READ-FIRST.md. Concatenate its five numbered UTF-8 text pieces without separators to recover the original416,772-byte production/full-from-fbf2a517.patch; SHA2569412792ee0b3ed9a1ac0f68bc83350a1721cd6db96df7415031731164a75ce67. Those pieces replace only the transport of the exact original full patch. All original metadata and incremental/instrumentation source remain unchanged.

No raw witness bytes, checkpoints, journals, assets, executables or native logs are in this source-only handoff. The preserved first compile failure and later26-pass result are distinct.
