# Record Finalized Flags

Used by `build/make/tools/record-finalized-flags` to add flags in
`build/make/tools/aconfig/exported_flag_check/non_api_flags_list.txt` (exported
flags which do not guard APIs) to `prebuilts/sdk/<level>/finalized-flags.txt`.

`finalized-flags.txt` is  used by
`build/make/tools/aconfig/convert_finalized_flags` to add the SDK check in the
aconfig flag accessor.

## Workflow

The following diagram illustrates the data flow for finalizing flags and generating SDK checks.

```text
/-----------------------------------------------------------------------------\
|                                                                             |
|   1: NON-API FLAGS (once at finalization)                                   |
|   +-----------------------+   +------------------------+                    |
|   | Flag + Release Config |   | non_api_flags_list.txt |                    |
|   |                       |   | (Non-API flags)        |                    |
|   +-----------+-----------+   +-----------+------------+                    |
|       |       |                           |                                 |
|       |       +-------------> v <---------+                                 |
|       |                       |                                             |
|       |           [ finalize_non_api_flags ]                                |
|       |                       |                                             |
|       |                       |                                             |
|       |                       |                                             |
|   2: API FLAGS (once at finalization)                                       |
|       |                       |                                             |
|       |                       |                                             |
|       v                       |                                             |
|   +-----------------------+   |   +-----------------------+                 |
|   | metalava              |   |   | finalized-flags.txt   |                 |
|   | (API & @FlaggedApi)   |   |   | (Previously Finalized)|                 |
|   +-----------+-----------+   |   +-----------+-----------+                 |
|               |               |               |                             |
|               +-------------> v <-------------+                             |
|                               |                                             |
|                   [ record_finalized_flags ]                                |
|                               |                                             |
|                               v                                             |
|                 +---------------------------+                               |
|                 |    finalized-flags.txt    |                               |
|                 | (prebuilts/sdk/<level>/)  |                               |
|                 +-------------+-------------+                               |
|                               |                                             |
|   3: ACONFIG CODEGEN (every build)                                          |
|                               |                                             |
|                               v                                             |
|                  [ convert_finalized_flags ]                                |
|                               |                                             |
|                               v                                             |
|                    ( Generated SDK Check )                                  |
|                      In Flag Accessor                                       |
|                                                                             |
\-----------------------------------------------------------------------------/
```