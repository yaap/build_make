# Aconfig Storage File Versioning

This document explains the versioning scheme for aconfig storage files. Aconfig
uses binary files to store flag information on device for fast read access at
runtime. To allow for format changes over time while maintaining backwards
compatibility, these storage files are versioned.

There are four types of aconfig storage files: * **Package Table
(`<container>.package.map`):** Mapping from package names to information about
the flags in that package (most importantly, starting offset for the flag
values). * **Flag Value (`<container>.val`):** Actual values for the flags. *
**Flag Table (`<container>.flag.map`):** Mapping from flag name to specific
offset for that flag, used for cross-container lookup. * **Flag Info
(`<container>.info`):** Contains info about the overrides for each flag, used by
aflags (for debugging).

All file types have a header at the beginning of the file, the first four bytes
of which are always a version number. This version number dictates the format of
the rest of the file.

## Version Logs

### Version 1: Initial New Storage

*   **Package Map:** Initial version. The header contains: `version`,
    `container`, `file_type`, `file_size`, `num_packages`, `bucket_offset`, and
    `node_offset`. Each node in the package map contains: `package_name`,
    `package_id`, `boolean_start_index`, and `next_offset`.
*   **Flag Value:** Initial version. This version only supports boolean flags.
    The header contains: `version`, `container`, `file_type`, `file_size`,
    `num_boolean_flags`, and `boolean_value_offset`. The contents are just a
    list of boolean values.
*   **Flag Table:** Initial version. The header contains: `version`,
    `container`, `file_type`, `file_size`, `num_flags`, `bucket_offset`, and
    `node_offset`. Each node in the flag map contains: `package_id`,
    `flag_name`, `flag_type`, `flag_index`, and `next_offset`.
*   **Flag Info:** Initial version. The header contains: `version`, `container`,
    `file_type`, `file_size`, `num_flags`, and `boolean_flag_offset`. Each node
    contains a single `attributes` field, which is a bitmask indicating if the
    flag is read-write, has a server override, or has a local override.

### Version 2: Fingerprint

Version 2 adds a fingerprint to the package, used to prevent stale offset-based
lookups.

*   **Package Map:** Added a 64-bit `fingerprint` to each node. This is a hash
    of the package's aconfig flags.

*No changes to the other files.*

### Version 3: Redaction

Version 3 adds redaction to the package, to prevent production builds relying on
exported flag values.

*   **Package Map:** Added a boolean `redact_exported_reads` to each node. This
    is used to prevent flag reads for exported flags in production builds.

*No changes to the other files.*

### Version 4: Integer Support

Version 4 adds int support to the storage files. This means the value files need
to contain a list of ints, and the package map needs to have both the int and
the boolean offset for its flags.

*   **Package Map:** Added an integer `int_start_index` to each node.
*   **Flag Value:** The header was extended to include `num_int_flags` and
    `int_value_offset`. The file now contains a list of integer values after the
    list of boolean values.
*   **Flag Info:** Similar to flag value, the header was extended to include
    `num_int_flags` and `int_flag_offset`. The nodes for int flag info are
    appended after the nodes for boolean. Note that unlike flag value, the
    structure of the flag info node are the same for boolean and int.
