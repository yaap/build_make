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

if [[ "$(readlink $TOP/prebuilts/sdk/latest)" == "$MAJOR.$MINOR" ]]; then
    info "finalize-platform-sdk: already done, exit early"
    return
fi

# introduce new SDK extension
apply_patches \
    packages/modules/SdkExtensions \
    "$BUNDLED_PATCHES"/packages/modules/SdkExtensions/0001-derive_sdk-introduce-C-extension.patch \
    "$BUNDLED_PATCHES"/packages/modules/SdkExtensions/0002-SdkExtensions-introduce-C-extension.patch \
    "$BUNDLED_PATCHES"/packages/modules/SdkExtensions/0003-sdk-extensions-info.xml-add-the-C-SDK-extensions-C-e.patch
apply_patches \
    packages/modules/common \
    "$BUNDLED_PATCHES"/packages/modules/common/0001-sdk.proto-add-C-modules-none.patch
apply_patches \
    frameworks/libs/modules-utils \
    "$BUNDLED_PATCHES"/frameworks/libs/modules-utils/0001-Add-isAtLeastC.patch

# update Build.java and aapt
apply_patches \
    frameworks/base \
    "$BUNDLED_PATCHES"/frameworks/base/0001-C-is-37.patch

# set SDK extension versions (or $SDK_EXT_VERSION won't appear in api-versions.xml)
for release_config in next trunk trunk_staging; do
    set_build_flags $release_config RELEASE_PLATFORM_SDK_EXTENSION_VERSION=$SDK_EXT_VERSION
done

# build the SDK
set_prospective_sdk_version_full "$MAJOR.$MINOR"
m sdk sdk_repo dist
clear_prospective_sdk_version_full

# populate prebuilts/sdk
#
# Will update these files under prebuilts/sdk/$MAJOR.$MINOR, and
# prebuilts/sdk/{Android.bp,latest}
m unpack-platform-sdk
unpack-platform-sdk \
    --dist-dir "$DIST_DIR" \
    --android-top "$TOP" \
    --version "$MAJOR.$MINOR"
git_commit \
    prebuilts/sdk \
<<EOF
Finalize platform SDK for Android $MAJOR.$MINOR

Files imported from ab/$BUILD_NUMBER.

Bug: $BUG
Test: N/A
Flag: NONE platform SDK finalization
EOF
