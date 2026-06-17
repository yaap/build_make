/*
 * Copyright (C) 2025 The Android Open Source Project
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

mod build_flags;
mod sdk_version;

#[cfg(test)]
mod tests {
    use crate::build_flags::{ReleaseConfigs, FLAGS_WE_CARE_ABOUT};
    use crate::sdk_version::SdkVersion;
    use regex::Regex;
    use std::sync::LazyLock;

    // the subset of build flags relevant for SDK finalization
    static RELEASE_CONFIGS: LazyLock<ReleaseConfigs> = LazyLock::new(ReleaseConfigs::init);

    fn sdk_version(release_config: &str) -> SdkVersion {
        // use SDK_INT_FULL if set, otherwise fall back to SDK_INT
        let s = &RELEASE_CONFIGS.flags[release_config]["RELEASE_PLATFORM_SDK_VERSION_FULL"];
        if !s.is_empty() {
            s.parse::<SdkVersion>().unwrap_or_else(|_| {
                panic!(
                    "failed to parse RELEASE_PLATFORM_SDK_VERSION_FULL for {release_config} ({s}) as SdkVersion"
                )
            })
        } else {
            let s = &RELEASE_CONFIGS.flags[release_config]["RELEASE_PLATFORM_SDK_VERSION"];
            s.parse::<SdkVersion>().unwrap_or_else(|_| {
                panic!(
                    "failed to parse RELEASE_PLATFORM_SDK_VERSION for {release_config} ({s}) as SdkVersion"
                )
            })
        }
    }

    #[test]
    fn test_build_flags_in_trunk_and_trunk_staging_are_equal() {
        // invariant: the values of the flags (that this test cares about) in RELEASE_CONFIGS.flags are equal
        // across trunk and trunk_staging release configs
        //
        // this means that the rest of the tests can focus on trunk and ignore trunk_staging
        for flag in FLAGS_WE_CARE_ABOUT {
            assert_eq!(
                RELEASE_CONFIGS.flags["trunk"][flag], RELEASE_CONFIGS.flags["trunk_staging"][flag],
                "flag {flag} differenct across trunk and trunk_staging",
            );
        }
    }

    #[test]
    fn test_trunk_is_never_rel() {
        // invariant: the codename in trunk is never REL: trunk is always bleeding edge and thus
        // always something later than the latest finalized (REL) platform
        assert_ne!(RELEASE_CONFIGS.flags["trunk"]["RELEASE_PLATFORM_VERSION_CODENAME"], "REL");
    }

    #[test]
    fn test_version_parity_if_next_is_not_rel() {
        // invariant: the version code of trunk and next are identical, unless next is REL: then
        // the version in trunk can be one less than the version in next (during the intermediate
        // state where next is REL but we haven't created prebuilts/sdk/<new-version> yet), or the
        // version in trunk is identical to the one in next
        let next = &RELEASE_CONFIGS.aliases["next"];
        if RELEASE_CONFIGS.flags[next]["RELEASE_PLATFORM_VERSION_CODENAME"] != "REL" {
            // expect the versions to be identical
            assert_eq!(
                RELEASE_CONFIGS.flags[next]["RELEASE_PLATFORM_SDK_VERSION_FULL"],
                RELEASE_CONFIGS.flags["trunk"]["RELEASE_PLATFORM_SDK_VERSION_FULL"]
            );
        } else {
            // make sure the version in trunk is less or equal to that of next
            //
            // ideally this should check that trunk is at most one version behind next, but we
            // can't tell what that means, so let's settle for the weaker guarantee of "less or
            // equal"
            assert!(sdk_version("trunk") <= sdk_version(next));
        }
    }

    #[test]
    fn test_version_and_version_full_parity() {
        // invariant: for the release configs that set RELEASE_PLATFORM_SDK_VERSION_FULL:
        //   - the value can be parsed as a float
        //   - the value contains a decimal separator
        //   - the value before the decimal separator is identical to RELEASE_PLATFORM_SDK_VERSION
        //     (e.g. 36.0 and 36)
        for release_config in RELEASE_CONFIGS.flags.keys() {
            let sdk_int_full =
                &RELEASE_CONFIGS.flags[release_config]["RELEASE_PLATFORM_SDK_VERSION_FULL"];
            if sdk_int_full.is_empty() {
                // skip this release config if it doesn't set RELEASE_PLATFORM_SDK_VERSION_FULL
                continue;
            }
            assert!(
                sdk_int_full.parse::<f32>().is_ok(),
                "failed to convert value ({sdk_int_full}) of RELEASE_PLATFORM_SDK_VERSION_FULL for {release_config} to f32"
            );
            let (integer_part, _) = sdk_int_full.split_once(".").unwrap_or_else(|| panic!("value of RELEASE_PLATFORM_SDK_VERSION_FULL ({sdk_int_full}) for {release_config} doesn't have expected format"));
            let sdk_int = &RELEASE_CONFIGS.flags[release_config]["RELEASE_PLATFORM_SDK_VERSION"];
            assert_eq!(
                integer_part, sdk_int,
                "in release config {release_config}, expected parity between the integer part ({integer_part}) of RELEASE_PLATFORM_SDK_VERSION_FULL ({sdk_int_full}) and RELEASE_PLATFORM_SDK_VERSION ({sdk_int})"
            );
        }
    }

    #[test]
    fn test_release_hidden_api_exportable_stubs_is_enabled_in_next() {
        // invariant: RELEASE_HIDDEN_API_EXPORTABLE_STUBS is set to `true` in `next`, because we'll
        // cut an Android release from this release config (the flag is too expensive in terms of
        // build performance to enable everywhere)
        let next = &RELEASE_CONFIGS.aliases["next"];
        let value = &RELEASE_CONFIGS.flags[next]["RELEASE_HIDDEN_API_EXPORTABLE_STUBS"];
        assert_eq!(
            value, "true",
            "expected RELEASE_HIDDEN_API_EXPORTABLE_STUBS to be 'true' in next ({next}) but was '{value}'"
        );
    }

    #[test]
    fn test_only_canary_release_config_has_codename_canary() {
        // invariant: only the canary release config ("canary", aliased to "zp11") sets codename to
        // CANARY; no release config inherits from the canary release config
        let canary = &RELEASE_CONFIGS.aliases["canary"];
        let value = &RELEASE_CONFIGS.flags[canary]["RELEASE_PLATFORM_VERSION_CODENAME"];
        assert_eq!(value, "CANARY");

        for release_config in RELEASE_CONFIGS.flags.keys().filter(|key| *key != canary) {
            let value = &RELEASE_CONFIGS.flags[release_config]["RELEASE_PLATFORM_VERSION_CODENAME"];
            assert_ne!(value, "CANARY");
        }
    }

    #[test]
    fn test_prospective_sdk_version() {
        // invariant: if set, RELEASE_PROSPECTIVE_SDK_VERSION_FULL is greater or equal to the
        // current SDK version (we never re-finalize older SDK versions)
        for release_config in RELEASE_CONFIGS.flags.keys() {
            let prospective_version = &RELEASE_CONFIGS.flags[release_config]
                ["RELEASE_PLATFORM_PROSPECTIVE_SDK_VERSION_FULL"];
            if !prospective_version.is_empty() {
                let prospective_version = prospective_version.parse::<SdkVersion>().unwrap();
                let sdk_version = sdk_version(release_config);
                assert!(prospective_version >= sdk_version, "in release config {release_config}, expected RELEASE_PROSPECTIVE_SDK_VERSION_FULL ({prospective_version}) to not be set, or to be greater or equal to the SDK version ({sdk_version})");
            }
        }
    }

    #[test]
    fn test_preview_sdk_int() {
        // invariants for the value of RELEASE_PLATFORM_PREVIEW_SDK_INT:
        //   - codename is CANARY: ${YYYY}${MM}${DD}
        //   - if codename is REL and this is not a beta release: 0
        //   - if codename is REL and this is a beta release: ${MM}${m}${b}
        //     (four chars, MM: major version, m: minor version, b: beta release number)
        //  - if codename is not REL: 1

        let re_release_config_beta = Regex::new(r"^.p.\d$").unwrap();
        let re_preview_sdk_int_canary = Regex::new(r"^(\d{4})(\d{2})(\d{2})$").unwrap();
        let re_preview_sdk_int_beta = Regex::new(r"^(\d{2})(\d)\d$").unwrap();
        for release_config in RELEASE_CONFIGS.flags.keys() {
            let codename =
                &RELEASE_CONFIGS.flags[release_config]["RELEASE_PLATFORM_VERSION_CODENAME"];
            let preview_sdk_int =
                &RELEASE_CONFIGS.flags[release_config]["RELEASE_PLATFORM_PREVIEW_SDK_INT"];

            if codename == "CANARY" {
                let error_msg = format!("in release config {release_config}, expected RELEASE_PLATFORM_PREVIEW_SDK_INT to be ${{YYYY}}${{MM}}${{DD}} but was {preview_sdk_int}");
                let Some(caps) = re_preview_sdk_int_canary.captures(preview_sdk_int) else {
                    panic!("{error_msg}");
                };
                let year = caps[1].parse::<u32>().unwrap();
                let month = caps[2].parse::<u32>().unwrap();
                let day = caps[3].parse::<u32>().unwrap();
                assert!((2026..=3000).contains(&year), "{error_msg}: unreasonable year");
                assert!((1..=12).contains(&month), "{error_msg}: unreasonable month");
                // not all months have 31 days, but this is good enough
                assert!((1..=31).contains(&day), "{error_msg}: unreasonable day");
            } else if codename == "REL" {
                let preview_sdk_int = preview_sdk_int.parse::<u32>().unwrap();
                assert_eq!(preview_sdk_int, 0, "in release config {release_config}, expected RELEASE_PLATFORM_PREVIEW_SDK_INT to be 0 but was {preview_sdk_int}");
            } else if re_release_config_beta.is_match(release_config) {
                let error_msg = format!("in release config {release_config}, expected RELEASE_PLATFORM_PREVIEW_SDK_INT to be ${{MM}}${{m}}${{b}} but was {preview_sdk_int}");
                let Some(caps) = re_preview_sdk_int_beta.captures(preview_sdk_int) else {
                    panic!("{error_msg}");
                };
                let beta = SdkVersion {
                    major: caps[1].parse::<u32>().unwrap(),
                    minor: caps[2].parse::<u32>().unwrap(),
                };
                let current = sdk_version(release_config);
                assert!((beta.major > current.major && beta.minor == 0) || (beta.major == current.major && beta.minor > current.minor), "in release config {release_config}, expected the release encoded in RELEASE_PLATFORM_PREVIEW_SDK_INT ({beta}) to encode a later release than RELEASE_PLATFORM_SDK_VERSION_FULL ({beta})");
            } else {
                let preview_sdk_int = preview_sdk_int.parse::<u32>().unwrap();
                assert_ne!(preview_sdk_int, 0, "in release config {release_config}, expected RELEASE_PLATFORM_PREVIEW_SDK_INT to be non-zero but was 0");
            }
        }
    }
}
