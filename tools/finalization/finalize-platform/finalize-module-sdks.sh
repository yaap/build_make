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

if test -e "$TOP/prebuilts/sdk/extensions/$SDK_EXT_VERSION"; then
    info "finalize-module-sdks: already done, exit early"
    return
fi

# The module SDKs do not exist already; build them
if [[ -z "$mainline_sdks_dir" ]]; then
    TARGET_RELEASE=sdk_finalization TARGET_BUILD_VARIANT=userdebug UNBUNDLED_BUILD_SDKS_FROM_SOURCE=true vendor/google/build/mainline_modules_sdks.sh --build-release next
    mainline_sdks_dir="$DIST_DIR"
fi

m unpack-module-sdks
projects="$(unpack-module-sdks \
    --mainline-sdks-top "$mainline_sdks_dir" \
    --android-top "$TOP" \
    --sdk-ext-version $SDK_EXT_VERSION)"

for project in $projects; do
    git_commit \
        $project \
<<EOF
Finalize SDK extension $SDK_EXT_VERSION

Import module SDK artifacts from ab/$BUILD_NUMBER.

Bug: $BUG
Test: N/A
Flag: NONE platform SDK finalization
EOF
done
