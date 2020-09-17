# v0.1.4

- Removed Versionize proc macro support for unions. Serializing unions can lead to undefined behaviour especially when no
layout guarantees are provided. The Versionize trait can still be implemented but only for repr(C) unions and extensive care
and testing is required from the implementer.

# v0.1.3

- Added extra validations in VersionMap::get_type_version().

# v0.1.2

- Improved edge cases handling for Vec serialization and deserialization.

# v0.1.0

- "versionize" v0.1.0 first release.
