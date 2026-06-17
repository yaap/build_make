#!/bin/bash
#
# Copyright (C) 2026 The Android Open Source Project
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#      http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

if [[ "$(build-flag --quiet --release=next get RELEASE_PLATFORM_SDK_VERSION_FULL)" == "$MAJOR.$MINOR" ]]; then
    info "finalize-release-configs: already done, exit early"
    return
fi

# Update SdkExtensions database
m gen_sdk
gen_sdk --database \
    "$TOP/packages/modules/SdkExtensions/gen_sdk/extensions_db.textpb" \
    --action new_sdk \
    --sdk $SDK_EXT_VERSION
git_commit \
    packages/modules/SdkExtensions \
<<EOF
Add new SDK extension version $SDK_EXT_VERSION

Bug: $BUG
Test: N/A
Flag: NONE platform SDK finalization
EOF

# Set SDK ZIP properties
pushd "$TOP/development"

cat >sdk/build_tools_source.prop_template <<EOF
Pkg.UserSrc=false
Pkg.Revision=\${PLATFORM_SDK_VERSION}.0.0
EOF

cat >sdk/platform_source.prop_template <<EOF
Pkg.Desc=Android SDK Platform \${PLATFORM_VERSION}
Pkg.UserSrc=false
Platform.Version=\${PLATFORM_VERSION}
Platform.CodeName=
Pkg.Revision=1
AndroidVersion.ApiLevel=\${PLATFORM_SDK_VERSION}
AndroidVersion.CodeName=\${PLATFORM_VERSION_CODENAME}
AndroidVersion.ExtensionLevel=\${PLATFORM_SDK_EXTENSION_VERSION}
AndroidVersion.IsBaseSdk=\${PLATFORM_IS_BASE_SDK}
AndroidVersion.PreviewSdkInt=\${PLATFORM_PREVIEW_SDK_VERSION}
AndroidVersion.BetaVersion=\${BETA_SDK_VERSION}
Layoutlib.Api=15
Layoutlib.Revision=1
Platform.MinToolsRev=22
EOF

popd
git_commit \
    development \
<<EOF
Set SDK zip properties for Android $MAJOR.$MINOR

Bug: $BUG
Test: presubmit
Flag: NONE platform SDK finalization
EOF

# Update hardcoded codename constants, API level mappings, etc
apply_patches \
    build/soong \
    "$BUNDLED_PATCHES"/build/soong/0001-C-is-37.patch
apply_patches \
    cts \
    "$BUNDLED_PATCHES"/cts/0001-C-is-37.patch
apply_patches \
    libcore \
    "$BUNDLED_PATCHES"/libcore/0001-C-is-37.patch \
    "$BUNDLED_PATCHES"/libcore/0002-Rename-VersionCodes.C-VersionCodes.CINNAMON_BUN.patch
apply_patches \
    platform_testing \
    "$BUNDLED_PATCHES"/platform_testing/0001-C-is-37.patch
apply_patches \
    tools/platform-compat \
    "$BUNDLED_PATCHES"/tools/platform-compat/0001-C-is-37.patch

# Set codename to REL and other build flags
set_build_flags next \
    RELEASE_PLATFORM_SDK_VERSION_FULL=$MAJOR.$MINOR \
    RELEASE_PLATFORM_SDK_VERSION=$MAJOR \
    RELEASE_PLATFORM_BASE_SDK_EXTENSION_VERSION=$SDK_EXT_VERSION \
    RELEASE_PLATFORM_VERSION_ALL_CODENAMES=REL \
    RELEASE_PLATFORM_VERSION_CODENAME=REL \
    RELEASE_PLATFORM_VERSION_ALL_PREVIEW_CODENAMES=REL \
    RELEASE_PLATFORM_PREVIEW_SDK_INT=0 \
    RELEASE_PLATFORM_VERSION_LAST_STABLE=$MARKETING_VERSION
for release_config in trunk trunk_staging; do
    set_build_flags $release_config \
        RELEASE_PLATFORM_SDK_VERSION_FULL=$MAJOR.$MINOR \
        RELEASE_PLATFORM_SDK_VERSION=$MAJOR \
        RELEASE_PLATFORM_BASE_SDK_EXTENSION_VERSION=$SDK_EXT_VERSION \
        RELEASE_PLATFORM_VERSION_LAST_STABLE=$MARKETING_VERSION
done
