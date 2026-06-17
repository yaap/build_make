/*
 * Copyright (C) 2026 The Android Open Source Project
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

use aconfig_protos::{ParsedFlagExt, ProtoFlagState, ProtoParsedFlags};
use std::collections::HashSet;

pub(crate) type FlagId = String;

pub(crate) fn extract_flags_from_cache(
    parsed_flags: ProtoParsedFlags,
    release_config: &str,
) -> HashSet<FlagId> {
    let source_substring = format!("/{}/", release_config);
    parsed_flags
        .parsed_flag
        .into_iter()
        .filter(|flag| {
            // Skip the flag if it's disabled.
            if flag.state() == ProtoFlagState::DISABLED {
                return false;
            }

            // Keep any flag that comes from the provided release config.
            flag.trace.iter().any(|tracepoint| tracepoint.source().contains(&source_substring))
        })
        .map(|flag| flag.fully_qualified_name())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use aconfig_protos::{ProtoParsedFlag, ProtoTracepoint};
    use std::collections::HashSet;

    fn create_test_flag(
        package: &str,
        name: &str,
        state: ProtoFlagState,
        sources: Vec<&str>,
    ) -> ProtoParsedFlag {
        let mut flag = ProtoParsedFlag::new();
        flag.set_package(package.to_string());
        flag.set_name(name.to_string());
        flag.set_state(state);
        let mut tracepoints = Vec::new();
        for source in sources {
            let mut tp = ProtoTracepoint::new();
            tp.set_source(source.to_string());
            tracepoints.push(tp);
        }
        flag.trace = tracepoints;
        flag
    }

    #[test]
    fn test_extract_flags() {
        let mut parsed_flags = ProtoParsedFlags::new();
        parsed_flags.parsed_flag.push(create_test_flag(
            "com.android",
            "enabled_in_trunk",
            ProtoFlagState::ENABLED,
            vec!["build/release/aconfig/trunk/com.android/enabled_in_trunk.textproto"],
        ));
        parsed_flags.parsed_flag.push(create_test_flag(
            "com.android",
            "disabled_in_trunk",
            ProtoFlagState::DISABLED,
            vec!["build/release/aconfig/trunk/com.android/disabled_in_trunk.textproto"],
        ));
        parsed_flags.parsed_flag.push(create_test_flag(
            "com.android",
            "enabled_in_trunk_staging",
            ProtoFlagState::ENABLED,
            vec!["build/release/aconfig/trunk_staging/com.android/enabled_in_trunk.textproto"],
        ));
        // Does not actually happen, but testing for completeness.
        parsed_flags.parsed_flag.push(create_test_flag(
            "com.android",
            "disabled_in_final",
            ProtoFlagState::DISABLED,
            vec!["build/release/aconfig/ap4a/com.android/disabled_in_final.textproto"],
        ));
        parsed_flags.parsed_flag.push(create_test_flag(
            "com.android",
            "disabled_mixed_sources",
            ProtoFlagState::DISABLED,
            vec![
                "build/release/aconfig/ap4a/com.android/disabled_mixed_sources.textproto",
                "build/release/aconfig/trunk/com.android/disabled_mixed_sources.textproto",
            ],
        ));
        parsed_flags.parsed_flag.push(create_test_flag(
            "com.android",
            "enabled_final",
            ProtoFlagState::ENABLED,
            vec!["build/release/aconfig/ap4a/com.android/enabled_final.textproto"],
        ));
        parsed_flags.parsed_flag.push(create_test_flag(
            "com.android",
            "enabled_mixed_sources",
            ProtoFlagState::ENABLED,
            vec![
                "build/release/aconfig/ap4a/com.android/enabled_mixed_sources.textproto",
                "build/release/aconfig/trunk/com.android/enabled_mixed_sources.textproto",
            ],
        ));
        parsed_flags.parsed_flag.push(create_test_flag(
            "com.android",
            "enabled_mixed_sources_vendor",
            ProtoFlagState::ENABLED,
            vec!["vendor/google/release/aconfig/cp4a/com.android/enabled_mixed_sources_vendor.textproto", "build/release/aconfig/trunk_staging/com.android/enabled_mixed_sources_vendor.textproto"],
        ));

        let extracted: HashSet<String> =
            extract_flags_from_cache(parsed_flags.clone(), "ap4a").into_iter().collect();
        let expected: HashSet<String> = HashSet::from([
            "com.android.enabled_mixed_sources".to_string(),
            "com.android.enabled_final".to_string(),
        ]);
        assert_eq!(extracted, expected);

        let extracted_cp4a = extract_flags_from_cache(parsed_flags, "cp4a");
        let expected_cp4a = HashSet::from(["com.android.enabled_mixed_sources_vendor".to_string()]);
        assert_eq!(extracted_cp4a, expected_cp4a);
    }

    #[test]
    fn test_does_not_extract_bad_flags() {
        let mut parsed_flags = ProtoParsedFlags::new();
        parsed_flags.parsed_flag.push(create_test_flag(
            "com.android",
            "invalid_rc_1",
            ProtoFlagState::ENABLED,
            vec!["build/release/aconfig/nonsensebp4a/com.android/invalid_rc_1.textproto"],
        ));
        parsed_flags.parsed_flag.push(create_test_flag(
            "com.android",
            "invalid_rc_2",
            ProtoFlagState::ENABLED,
            vec!["build/release/aconfig/bp4anonsense/com.android/invalid_rc_2.textproto"],
        ));
        let extracted: HashSet<String> =
            extract_flags_from_cache(parsed_flags.clone(), "bp4a").into_iter().collect();
        assert_eq!(extracted, HashSet::new());
    }
}
