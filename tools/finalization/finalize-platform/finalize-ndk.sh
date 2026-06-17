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

if test -e "$TOP/prebuilts/abi-dumps/ndk/$MAJOR"; then
    info "finalize-ndk: already done, exit early"
    return
fi

# TODO: Use sdk_finalization ?
TARGET_RELEASE=trunk_staging

set_build_flags $TARGET_RELEASE \
    RELEASE_PLATFORM_SDK_VERSION=$MAJOR \
    RELEASE_PLATFORM_SDK_VERSION_FULL=$MAJOR.$MINOR

$TOP/development/tools/ndk/update_ndk_abi.sh
git_commit prebuilts/abi-dumps/ndk <<EOF
Record the NDK APIs for $MAJOR

Generated with development/tools/ndk/update_ndk_abi.sh with
RELEASE_PLATFORM_SDK_VERSION set to $MAJOR for $TARGET_RELEASE

Bug: $BUG
Test: N/A
Flag: NONE platform SDK finalization
EOF

m create_reference_dumps
create_reference_dumps \
    --release $TARGET_RELEASE \
    --build-variant $TARGET_BUILD_VARIANT \
    --lib-variant APEX
git_commit prebuilts/abi-dumps/platform <<EOF
Create reference dumps for $MAJOR

Files created via
create_reference_dumps \\
    --release $TARGET_RELEASE \\
    --build-variant $TARGET_BUILD_VARIANT \\
    --lib-variant APEX

Bug: $BUG
Test: N/A
Flag: NONE platform SDK finalization
EOF

project=$(get_project_for_release $TARGET_RELEASE)
pushd $TOP/$project
git checkout goog/main
git branch -D $BRANCH
popd
