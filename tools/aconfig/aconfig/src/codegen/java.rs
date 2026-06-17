/*
* Copyright (C) 2023 The Android Open Source Project
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

use anyhow::{ensure, Result};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use tinytemplate::TinyTemplate;

use crate::codegen::{self, get_flag_offset_in_storage_file, CodegenMode};
use crate::commands::OutputFile;
use aconfig_protos::{
    ProtoFlagPermission, ProtoFlagState, ProtoFlagStorageBackend, ProtoParsedFlag,
};
use convert_finalized_flags::{ApiLevel, FinalizedFlag, FinalizedFlagMap};
use std::collections::HashMap;

// Arguments to configure codegen for generate_java_code.
#[derive(Default)]
pub struct JavaCodegenConfig {
    pub codegen_mode: CodegenMode,
    pub flag_ids: HashMap<String, u16>,
    pub package_fingerprint: u64,
    pub single_exported_file: bool,
    pub finalized_flags: FinalizedFlagMap,
    // Whether to add the "@UnsupportedAppUsage" (UAU) annotation in the generated code.
    pub support_uau_annotation: bool,
    // Whether to optimize read-only flag reads by short-circuiting test override support.
    pub optimize_read_only_getter: bool,
    // Whether to allow streamlining and/or omission of the internal impl interface for
    // supported read-only scenarios.
    pub allow_impl_interface_removal: bool,
    pub generate_checks_sdk_annotation: bool,
}

pub fn generate_java_code<I>(
    package: &str,
    parsed_flags_iter: I,
    config: JavaCodegenConfig,
) -> Result<Vec<OutputFile>>
where
    I: Iterator<Item = ProtoParsedFlag>,
{
    let mut use_device_config = false;
    let mut use_aconfigd = false;
    let flag_elements: Vec<FlagElement> = parsed_flags_iter
        .map(|pf| {
            use_device_config |= pf.metadata.storage() == ProtoFlagStorageBackend::DEVICE_CONFIG;
            use_aconfigd |= pf.metadata.storage() == ProtoFlagStorageBackend::ACONFIGD;
            ensure!(
                !(use_device_config && use_aconfigd),
                "Package {} cannot contain both device_config and new storage stored flags",
                package
            );

            create_flag_element(package, &pf, config.flag_ids.clone(), &config.finalized_flags)
        })
        .collect::<Result<Vec<FlagElement>>>()?;
    let namespace_flags = gen_flags_by_namespace(&flag_elements);
    let properties_set: BTreeSet<String> =
        flag_elements.iter().map(|fe| format_property_name(&fe.device_config_namespace)).collect();
    let is_test_mode = config.codegen_mode == CodegenMode::Test;
    let library_exported = config.codegen_mode == CodegenMode::Exported;
    let runtime_lookup_required =
        flag_elements.iter().any(|elem| elem.is_read_write) || library_exported;
    let container = (flag_elements.first().expect("zero template flags").container).to_string();
    let is_platform_container =
        matches!(container.as_str(), "system" | "system_ext" | "product" | "vendor");
    let preserve_impl_interface = !config.allow_impl_interface_removal;
    // Flags that are overridable and should be preserved in the FeatureFlags API.
    // In practice, we only omit flags when:
    //      1) the flag is read-only,
    //      2) read-only getter optimization is enabled, and
    //      3) the impl interface is not explicitly preserved for the current target.
    let overridable_flag_elements: Vec<FlagElement> = flag_elements
        .iter()
        .filter(|fe| {
            fe.is_read_write || !config.optimize_read_only_getter || preserve_impl_interface
        })
        .cloned()
        .collect();
    let is_read_only_optimized = overridable_flag_elements.is_empty()
        && !is_test_mode
        && !preserve_impl_interface
        && !runtime_lookup_required;
    let context = Context {
        flag_elements,
        overridable_flag_elements,
        namespace_flags,
        is_test_mode,
        runtime_lookup_required,
        properties_set,
        package_name: package.to_string(),
        library_exported,
        container,
        is_platform_container,
        package_fingerprint: format!("0x{:X}L", config.package_fingerprint),
        single_exported_file: config.single_exported_file,
        preserve_impl_interface,
        use_device_config,
        support_uau_annotation: config.support_uau_annotation,
        optimize_read_only_getter: config.optimize_read_only_getter,
        generate_checks_sdk_annotation: config.generate_checks_sdk_annotation,
        is_read_only_optimized,
    };
    let mut template = TinyTemplate::new();
    if !is_read_only_optimized {
        if library_exported && config.single_exported_file {
            template.add_template(
                "ExportedFlags.java",
                include_str!("../../templates/ExportedFlags.java.template"),
            )?;
        } else {
            template.add_template(
                "CustomFeatureFlags.java",
                include_str!("../../templates/CustomFeatureFlags.java.template"),
            )?;
            template.add_template(
                "FakeFeatureFlagsImpl.java",
                include_str!("../../templates/FakeFeatureFlagsImpl.java.template"),
            )?;
        }
        add_feature_flags_impl_template(&context, &mut template)?;
        template.add_template(
            "FeatureFlags.java",
            include_str!("../../templates/FeatureFlags.java.template"),
        )?;
    }
    template.add_template("Flags.java", include_str!("../../templates/Flags.java.template"))?;

    let path: PathBuf = package.split('.').collect();
    let mut files = vec!["Flags.java"];
    if !is_read_only_optimized {
        files.extend(vec!["FeatureFlags.java", "FeatureFlagsImpl.java"]);
        if library_exported && config.single_exported_file {
            files.push("ExportedFlags.java");
        } else {
            files.push("CustomFeatureFlags.java");
            files.push("FakeFeatureFlagsImpl.java");
        }
    }
    files
        .iter()
        .map(|file| {
            Ok(OutputFile {
                contents: template.render(file, &context)?.into(),
                path: path.join(file),
            })
        })
        .collect::<Result<Vec<OutputFile>>>()
}

fn gen_flags_by_namespace(flags: &[FlagElement]) -> Vec<NamespaceFlags> {
    let mut namespace_to_flag: BTreeMap<String, Vec<FlagElement>> = BTreeMap::new();

    for flag in flags {
        match namespace_to_flag.get_mut(&flag.device_config_namespace) {
            Some(flag_list) => flag_list.push(flag.clone()),
            None => {
                namespace_to_flag.insert(flag.device_config_namespace.clone(), vec![flag.clone()]);
            }
        }
    }

    namespace_to_flag
        .iter()
        .map(|(namespace, flags)| NamespaceFlags {
            namespace: namespace.to_string(),
            flags: flags.clone(),
        })
        .collect()
}

#[derive(Serialize)]
struct Context {
    pub flag_elements: Vec<FlagElement>,
    pub overridable_flag_elements: Vec<FlagElement>,
    pub namespace_flags: Vec<NamespaceFlags>,
    pub is_test_mode: bool,
    pub runtime_lookup_required: bool,
    pub properties_set: BTreeSet<String>,
    pub package_name: String,
    pub library_exported: bool,
    pub container: String,
    pub is_platform_container: bool,
    pub package_fingerprint: String,
    pub single_exported_file: bool,
    pub preserve_impl_interface: bool,
    pub use_device_config: bool,
    pub support_uau_annotation: bool,
    pub optimize_read_only_getter: bool,
    pub generate_checks_sdk_annotation: bool,
    pub is_read_only_optimized: bool,
}

#[derive(Serialize, Debug)]
struct NamespaceFlags {
    pub namespace: String,
    pub flags: Vec<FlagElement>,
}

#[derive(Serialize, Clone, Debug)]
struct FlagElement {
    pub container: String,
    pub default_value: bool,
    pub device_config_namespace: String,
    pub device_config_flag: String,
    pub flag_name: String,
    pub flag_name_constant_suffix: String,
    pub flag_offset: u16,
    pub is_read_write: bool,
    pub method_name: String,
    pub properties: String,
    pub finalized_sdk_present: bool,
    pub finalized_sdk_check: String,
    pub finalized_sdk_annotation: String,
}

fn create_flag_element(
    package: &str,
    pf: &ProtoParsedFlag,
    flag_ids: HashMap<String, u16>,
    finalized_flags: &FinalizedFlagMap,
) -> Result<FlagElement> {
    let device_config_flag = codegen::create_device_config_ident(package, pf.name())
        .expect("values checked at flag parse time");

    // An empty map is provided if check_api_level is disabled.
    let (finalized_sdk_present, finalized_sdk_value) = if !finalized_flags.is_empty() {
        let finalized_sdk = finalized_flags.get_finalized_level(&FinalizedFlag {
            flag_name: pf.name().to_string(),
            package_name: package.to_string(),
        });
        (finalized_sdk.is_some(), finalized_sdk.unwrap_or(ApiLevel(0)))
    } else {
        (false, ApiLevel(0))
    };
    let finalized_sdk_check = finalized_sdk_value.conditional();
    let finalized_sdk_annotation = finalized_sdk_value.annotation();

    Ok(FlagElement {
        container: pf.container().to_string(),
        default_value: pf.state() == ProtoFlagState::ENABLED,
        device_config_namespace: pf.namespace().to_string(),
        device_config_flag,
        flag_name: pf.name().to_string(),
        flag_name_constant_suffix: pf.name().to_ascii_uppercase(),
        flag_offset: get_flag_offset_in_storage_file(&flag_ids, pf)?,
        is_read_write: pf.permission() == ProtoFlagPermission::READ_WRITE,
        method_name: format_java_method_name(pf.name()),
        properties: format_property_name(pf.namespace()),
        finalized_sdk_present,
        finalized_sdk_check,
        finalized_sdk_annotation,
    })
}

fn format_java_method_name(flag_name: &str) -> String {
    let splits: Vec<&str> = flag_name.split('_').filter(|&word| !word.is_empty()).collect();
    if splits.len() == 1 {
        let name = splits[0];
        name[0..1].to_ascii_lowercase() + &name[1..]
    } else {
        splits
            .iter()
            .enumerate()
            .map(|(index, word)| {
                if index == 0 {
                    word.to_ascii_lowercase()
                } else {
                    word[0..1].to_ascii_uppercase() + &word[1..].to_ascii_lowercase()
                }
            })
            .collect::<Vec<String>>()
            .join("")
    }
}

fn format_property_name(property_name: &str) -> String {
    let name = format_java_method_name(property_name);
    format!("mProperties{}{}", &name[0..1].to_ascii_uppercase(), &name[1..])
}

fn add_feature_flags_impl_template(context: &Context, template: &mut TinyTemplate) -> Result<()> {
    if context.is_test_mode {
        // Test mode has its own template, so use regardless of any other settings.
        template.add_template(
            "FeatureFlagsImpl.java",
            include_str!("../../templates/FeatureFlagsImpl.test_mode.java.template"),
        )?;
        return Ok(());
    }

    match context.library_exported {
        // Exported library with new_exported enabled, use new storage exported template.
        true => {
            ensure!(
                !context.use_device_config,
                "All exported mode codegen should rely on new storage for safety"
            );
            template.add_template(
                "FeatureFlagsImpl.java",
                include_str!("../../templates/FeatureFlagsImpl.exported.java.template"),
            )?;
        }
        // New storage internal mode.
        false => match context.use_device_config {
            true => {
                template.add_template(
                    "FeatureFlagsImpl.java",
                    include_str!(
                        "../../templates/FeatureFlagsImpl.legacy_flag.internal.java.template"
                    ),
                )?;
            }
            false => {
                template.add_template(
                    "FeatureFlagsImpl.java",
                    include_str!("../../templates/FeatureFlagsImpl.new_storage.java.template"),
                )?;
            }
        },
    };
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::assign_flag_ids;
    use crate::test::first_significant_code_diff;
    use convert_finalized_flags::ApiLevel;
    use std::collections::{HashMap, HashSet};
    use std::fs;
    use std::path::{Path, PathBuf};
    use walkdir::WalkDir;

    const GOLDEN_SUFFIX: &str = ".golden";

    // Finds all `.golden` files in a dir, returning their relative paths stripped of the `.golden` suffix.
    fn find_golden_files(base_dir: &Path) -> Result<HashSet<PathBuf>, std::io::Error> {
        let mut files = HashSet::new();
        for entry in WalkDir::new(base_dir).into_iter().filter_map(Result::ok) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let relative_path = match path.strip_prefix(base_dir) {
                Ok(p) => p.to_path_buf(),
                Err(_) => continue,
            };

            if let Some(relative_str) = relative_path.to_str() {
                if relative_str.starts_with('.') || !relative_str.ends_with(GOLDEN_SUFFIX) {
                    continue;
                }

                if let Some(stripped) = relative_str.strip_suffix(GOLDEN_SUFFIX) {
                    files.insert(PathBuf::from(stripped));
                }
            }
        }
        Ok(files)
    }

    fn compare_with_goldens(test_name: &str, generated_files: Vec<OutputFile>) {
        let golden_root = PathBuf::from("tests/golden/java");
        let golden_dir = golden_root.join(test_name);
        let update_guidance = format!(
            "Run atest with ACONFIG_UPDATE_JAVA_GOLDENS_PATH=${{ANDROID_BUILD_TOP}}/build/tools/aconfig/aconfig/{} to update goldens.",
            golden_root.display()
        );

        if let Ok(update_path) = std::env::var("ACONFIG_UPDATE_JAVA_GOLDENS_PATH") {
            let update_dir = PathBuf::from(update_path).join(test_name);
            if update_dir.exists() {
                fs::remove_dir_all(&update_dir).unwrap();
            }
            fs::create_dir_all(&update_dir).unwrap();
            for file in &generated_files {
                let output_path = update_dir.join(&file.path);
                let golden_path = output_path.with_extension(format!(
                    "{}.{}",
                    output_path.extension().unwrap_or_default().to_str().unwrap_or(""),
                    GOLDEN_SUFFIX.trim_start_matches('.')
                ));
                fs::create_dir_all(golden_path.parent().unwrap()).unwrap();
                fs::write(golden_path, &file.contents).unwrap();
            }
            println!("Golden files for {} written to {:?}.", test_name, update_dir);
            return;
        }

        assert!(
            golden_dir.exists(),
            "Golden directory not found: {:?}\n{}",
            golden_dir,
            update_guidance
        );

        let golden_files = find_golden_files(&golden_dir).unwrap_or_else(|e| {
            panic!("Failed to read golden directory {:?}: {}\n{}", golden_dir, e, update_guidance);
        });

        let generated_file_map: HashMap<PathBuf, String> = generated_files
            .into_iter()
            .map(|f| (f.path, String::from_utf8(f.contents).unwrap()))
            .collect();

        let golden_keys: HashSet<_> = golden_files.iter().cloned().collect();
        let generated_keys: HashSet<_> = generated_file_map.keys().cloned().collect();

        let missing_in_generated: Vec<_> = golden_keys.difference(&generated_keys).collect();
        assert!(
            missing_in_generated.is_empty(),
            "Golden files found that were not generated: {:?}\n{}",
            missing_in_generated,
            update_guidance
        );

        let extra_in_generated: Vec<_> = generated_keys.difference(&golden_keys).collect();
        assert!(
            extra_in_generated.is_empty(),
            "Generated files found that have no corresponding golden file: {:?}\n{}",
            extra_in_generated,
            update_guidance
        );

        for golden_file_rel_path in &golden_files {
            let golden_file_abs_path = golden_dir.join(golden_file_rel_path);
            let expected_golden_path = golden_file_abs_path.with_extension(format!(
                "{}.{}",
                golden_file_abs_path.extension().unwrap_or_default().to_str().unwrap_or(""),
                GOLDEN_SUFFIX.trim_start_matches('.')
            ));
            let expected_content = fs::read_to_string(&expected_golden_path).unwrap_or_else(|e| {
                panic!(
                    "Failed to read golden file {:?}: {}\n{}",
                    expected_golden_path, e, update_guidance
                );
            });

            let generated_content = generated_file_map.get(golden_file_rel_path).unwrap();
            if let Some(diff) = first_significant_code_diff(&expected_content, generated_content) {
                panic!(
                    "Golden file content mismatch for {:?}\nDiff: {}\n{}",
                    golden_file_rel_path, diff, update_guidance
                );
            }
        }
    }

    fn run_generate_java_code_production_test(
        test_name: &str,
        optimize_read_only_getter: bool,
        allow_impl_interface_removal: bool,
    ) {
        let parsed_flags = crate::test::parse_test_flags();
        let mode = CodegenMode::Production;
        let modified_parsed_flags =
            crate::commands::modify_parsed_flags_based_on_mode(parsed_flags, mode).unwrap();
        let flag_ids =
            assign_flag_ids(crate::test::TEST_PACKAGE, modified_parsed_flags.iter()).unwrap();
        let config = JavaCodegenConfig {
            codegen_mode: mode,
            flag_ids,
            package_fingerprint: 5801144784618221668,
            optimize_read_only_getter,
            allow_impl_interface_removal,
            ..Default::default()
        };
        let generated_files = generate_java_code(
            crate::test::TEST_PACKAGE,
            modified_parsed_flags.into_iter(),
            config,
        )
        .unwrap();
        compare_with_goldens(test_name, generated_files);
    }

    #[test]
    fn test_generate_java_code_production() {
        let optimize_read_only_getter = false;
        let allow_impl_interface_removal = false;
        run_generate_java_code_production_test(
            "test_generate_java_code_production",
            optimize_read_only_getter,
            allow_impl_interface_removal,
        );
    }

    #[test]
    fn test_generate_java_code_production_allow_impl_removal() {
        let optimize_read_only_getter = false;
        let allow_impl_interface_removal = true;
        run_generate_java_code_production_test(
            "test_generate_java_code_production_allow_impl_removal",
            optimize_read_only_getter,
            allow_impl_interface_removal,
        );
    }

    #[test]
    fn test_generate_java_code_production_optimize_ro() {
        let optimize_read_only_getter = true;
        let allow_impl_interface_removal = false;
        run_generate_java_code_production_test(
            "test_generate_java_code_production_optimize_ro",
            optimize_read_only_getter,
            allow_impl_interface_removal,
        );
    }

    #[test]
    fn test_generate_java_code_production_optimize_ro_allow_impl_removal() {
        let optimize_read_only_getter = true;
        let allow_impl_interface_removal = true;
        run_generate_java_code_production_test(
            "test_generate_java_code_production_optimize_ro_allow_impl_removal",
            optimize_read_only_getter,
            allow_impl_interface_removal,
        );
    }

    #[test]
    fn test_generate_java_code_mainline_beta_production() {
        let parsed_flags = crate::test::parse_test_flags();
        let mode = CodegenMode::Production;
        let modified_parsed_flags: Vec<_> =
            crate::commands::modify_parsed_flags_based_on_mode(parsed_flags, mode)
                .unwrap()
                .into_iter()
                .map(|mut pf| {
                    if pf.metadata.storage() == ProtoFlagStorageBackend::ACONFIGD {
                        let m = pf.metadata.as_mut().unwrap();
                        m.set_storage(ProtoFlagStorageBackend::DEVICE_CONFIG);
                    }
                    pf
                })
                .collect();
        let flag_ids =
            assign_flag_ids(crate::test::TEST_PACKAGE, modified_parsed_flags.iter()).unwrap();
        let config = JavaCodegenConfig {
            codegen_mode: mode,
            flag_ids,
            package_fingerprint: 5801144784618221668,
            ..Default::default()
        };
        let generated_files = generate_java_code(
            crate::test::TEST_PACKAGE,
            modified_parsed_flags.into_iter(),
            config,
        )
        .unwrap();
        compare_with_goldens("test_generate_java_code_mainline_beta_production", generated_files);
    }

    #[test]
    fn test_generate_java_code_new_exported() {
        let parsed_flags = crate::test::parse_test_flags();
        let mode = CodegenMode::Exported;
        let modified_parsed_flags =
            crate::commands::modify_parsed_flags_based_on_mode(parsed_flags, mode).unwrap();
        let flag_ids =
            assign_flag_ids(crate::test::TEST_PACKAGE, modified_parsed_flags.iter()).unwrap();
        let config = JavaCodegenConfig {
            codegen_mode: mode,
            flag_ids,
            package_fingerprint: 5801144784618221668,
            ..Default::default()
        };
        let generated_files = generate_java_code(
            crate::test::TEST_PACKAGE,
            modified_parsed_flags.into_iter(),
            config,
        )
        .unwrap();

        compare_with_goldens("test_generate_java_code_new_exported", generated_files);
    }

    #[test]
    fn test_generate_java_code_new_exported_with_sdk_check() {
        let parsed_flags = crate::test::parse_test_flags();
        let mode = CodegenMode::Exported;
        let modified_parsed_flags =
            crate::commands::modify_parsed_flags_based_on_mode(parsed_flags, mode).unwrap();
        let flag_ids =
            assign_flag_ids(crate::test::TEST_PACKAGE, modified_parsed_flags.iter()).unwrap();
        let mut finalized_flags = FinalizedFlagMap::new();
        finalized_flags.insert_if_new(
            ApiLevel::from_sdk_int(36),
            FinalizedFlag {
                flag_name: "disabled_rw_exported".to_string(),
                package_name: "com.android.aconfig.test".to_string(),
            },
        );
        let config = JavaCodegenConfig {
            codegen_mode: mode,
            flag_ids,
            package_fingerprint: 5801144784618221668,
            finalized_flags,
            ..Default::default()
        };
        let generated_files = generate_java_code(
            crate::test::TEST_PACKAGE,
            modified_parsed_flags.into_iter(),
            config,
        )
        .unwrap();

        compare_with_goldens(
            "test_generate_java_code_new_exported_with_sdk_check",
            generated_files,
        );
    }

    // Test that the SDK check isn't added unless the library is exported (even
    // if the flag is present in finalized_flags).
    #[test]
    fn test_generate_java_code_flags_with_sdk_check() {
        let parsed_flags = crate::test::parse_test_flags();
        let mode = CodegenMode::Production;
        let modified_parsed_flags =
            crate::commands::modify_parsed_flags_based_on_mode(parsed_flags, mode).unwrap();
        let flag_ids =
            assign_flag_ids(crate::test::TEST_PACKAGE, modified_parsed_flags.iter()).unwrap();
        let mut finalized_flags = FinalizedFlagMap::new();
        finalized_flags.insert_if_new(
            ApiLevel::from_sdk_int(36),
            FinalizedFlag {
                flag_name: "disabled_rw".to_string(),
                package_name: "com.android.aconfig.test".to_string(),
            },
        );
        let config = JavaCodegenConfig {
            codegen_mode: mode,
            flag_ids,
            package_fingerprint: 5801144784618221668,
            finalized_flags,
            ..Default::default()
        };
        let generated_files = generate_java_code(
            crate::test::TEST_PACKAGE,
            modified_parsed_flags.into_iter(),
            config,
        )
        .unwrap();

        let test_name = "test_generate_java_code_flags_with_sdk_check";
        let file = generated_files.iter().find(|f| f.path.ends_with("Flags.java")).unwrap();
        compare_with_goldens(test_name, vec![(*file).clone()]);
    }

    #[test]
    fn test_generate_java_code_test() {
        let parsed_flags = crate::test::parse_test_flags();
        let mode = CodegenMode::Test;
        let modified_parsed_flags =
            crate::commands::modify_parsed_flags_based_on_mode(parsed_flags, mode).unwrap();
        let flag_ids =
            assign_flag_ids(crate::test::TEST_PACKAGE, modified_parsed_flags.iter()).unwrap();
        let config = JavaCodegenConfig {
            codegen_mode: mode,
            flag_ids,
            package_fingerprint: 5801144784618221668,
            ..Default::default()
        };
        let generated_files = generate_java_code(
            crate::test::TEST_PACKAGE,
            modified_parsed_flags.into_iter(),
            config,
        )
        .unwrap();

        compare_with_goldens("test_generate_java_code_test", generated_files);
    }

    #[test]
    fn test_generate_java_code_force_read_only() {
        let parsed_flags = crate::test::parse_test_flags();
        let mode = CodegenMode::ForceReadOnly;
        let modified_parsed_flags =
            crate::commands::modify_parsed_flags_based_on_mode(parsed_flags, mode).unwrap();
        let flag_ids =
            assign_flag_ids(crate::test::TEST_PACKAGE, modified_parsed_flags.iter()).unwrap();
        let config = JavaCodegenConfig {
            codegen_mode: mode,
            flag_ids,
            package_fingerprint: 5801144784618221668,
            ..Default::default()
        };
        let generated_files = generate_java_code(
            crate::test::TEST_PACKAGE,
            modified_parsed_flags.into_iter(),
            config,
        )
        .unwrap();
        compare_with_goldens("test_generate_java_code_force_read_only", generated_files);
    }

    #[test]
    fn test_generate_java_code_exported_flags() {
        // TODO(b/324592443): Clean this up once generate_checks_sdk_annotation is enabled by default.
        let mut golden_test_name: String = String::from("test_generate_java_code_exported_flags");
        if cfg!(feature = "generate_checks_sdk_annotation") {
            golden_test_name += "-annotated";
        }
        let parsed_flags = crate::test::parse_test_flags();
        let mode = CodegenMode::Exported;
        let modified_parsed_flags =
            crate::commands::modify_parsed_flags_based_on_mode(parsed_flags, mode).unwrap();
        let flag_ids =
            assign_flag_ids(crate::test::TEST_PACKAGE, modified_parsed_flags.iter()).unwrap();
        let mut finalized_flags = FinalizedFlagMap::new();
        finalized_flags.insert_if_new(
            ApiLevel::from_sdk_int(36),
            FinalizedFlag {
                flag_name: "disabled_rw_exported".to_string(),
                package_name: "com.android.aconfig.test".to_string(),
            },
        );
        let generate_checks_sdk_annotation = cfg!(feature = "generate_checks_sdk_annotation");

        let config = JavaCodegenConfig {
            codegen_mode: mode,
            flag_ids,
            package_fingerprint: 5801144784618221668,
            single_exported_file: true,
            finalized_flags,
            generate_checks_sdk_annotation,
            ..Default::default()
        };
        let generated_files = generate_java_code(
            crate::test::TEST_PACKAGE,
            modified_parsed_flags.into_iter(),
            config,
        )
        .unwrap();

        compare_with_goldens(&golden_test_name, generated_files);
    }

    #[test]
    fn test_mix_device_config_and_new_storage_flags() {
        let mut parsed_flags = crate::test::parse_test_flags();
        parsed_flags.parsed_flag[0].set_permission(ProtoFlagPermission::READ_WRITE);
        let m = parsed_flags.parsed_flag[0].metadata.as_mut().unwrap();
        m.set_storage(ProtoFlagStorageBackend::DEVICE_CONFIG);
        parsed_flags.parsed_flag[1].set_permission(ProtoFlagPermission::READ_WRITE);
        let m = parsed_flags.parsed_flag[1].metadata.as_mut().unwrap();
        m.set_storage(ProtoFlagStorageBackend::ACONFIGD);

        let flag_ids =
            assign_flag_ids(crate::test::TEST_PACKAGE, parsed_flags.parsed_flag.iter()).unwrap();

        let config = JavaCodegenConfig {
            codegen_mode: CodegenMode::Production,
            flag_ids,
            package_fingerprint: 5801144784618221668,
            ..Default::default()
        };
        let error = generate_java_code(
            crate::test::TEST_PACKAGE,
            parsed_flags.parsed_flag.into_iter(),
            config,
        )
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            "Package com.android.aconfig.test cannot contain both device_config and new storage stored flags",
        );
    }

    #[test]
    fn test_generate_java_code_force_read_only_optimized() {
        let parsed_flags = crate::test::parse_test_flags();
        let mode = CodegenMode::ForceReadOnly;
        let modified_parsed_flags =
            crate::commands::modify_parsed_flags_based_on_mode(parsed_flags, mode).unwrap();
        let flag_ids =
            assign_flag_ids(crate::test::TEST_PACKAGE, modified_parsed_flags.iter()).unwrap();
        let config = JavaCodegenConfig {
            codegen_mode: mode,
            flag_ids,
            package_fingerprint: 5801144784618221668,
            optimize_read_only_getter: true,
            allow_impl_interface_removal: true,
            ..Default::default()
        };
        let generated_files = generate_java_code(
            crate::test::TEST_PACKAGE,
            modified_parsed_flags.into_iter(),
            config,
        )
        .unwrap();

        assert_eq!(generated_files.len(), 1);
        assert_eq!(
            generated_files[0].path.to_str().unwrap(),
            "com/android/aconfig/test/Flags.java"
        );

        compare_with_goldens("test_generate_java_code_force_read_only_optimized", generated_files);
    }

    #[test]
    fn test_format_java_method_name() {
        let expected = "someSnakeName";
        let input = "____some_snake___name____";
        let formatted_name = format_java_method_name(input);
        assert_eq!(expected, formatted_name);

        let input = "someSnakeName";
        let formatted_name = format_java_method_name(input);
        assert_eq!(expected, formatted_name);

        let input = "SomeSnakeName";
        let formatted_name = format_java_method_name(input);
        assert_eq!(expected, formatted_name);

        let input = "SomeSnakeName_";
        let formatted_name = format_java_method_name(input);
        assert_eq!(expected, formatted_name);

        let input = "_SomeSnakeName";
        let formatted_name = format_java_method_name(input);
        assert_eq!(expected, formatted_name);
    }

    #[test]
    fn test_format_property_name() {
        let expected = "mPropertiesSomeSnakeName";
        let input = "____some_snake___name____";
        let formatted_name = format_property_name(input);
        assert_eq!(expected, formatted_name);

        let input = "someSnakeName";
        let formatted_name = format_property_name(input);
        assert_eq!(expected, formatted_name);

        let input = "SomeSnakeName";
        let formatted_name = format_property_name(input);
        assert_eq!(expected, formatted_name);

        let input = "SomeSnakeName_";
        let formatted_name = format_property_name(input);
        assert_eq!(expected, formatted_name);
    }
}
