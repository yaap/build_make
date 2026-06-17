Note to readers:

Please be aware that the presence of a flag in non_api_flags_list.txt does not
guarantee it will work correctly. This list is maintained for historical
purposes.
For an exported flag to function, it must be included in the
convert_finalized_flags list for the appropriate SDK level. Currently, the only
mechanism to add flags to that list is during the finalization process for API
flags. Consequently, creating an exemption for an exported flag is not possible
at this time.