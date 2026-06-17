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

# Print a debug message
function info() {
    local timestamp="$(date +'%Y-%m-%d %H:%M:%S')"
    if [[ -t 1 ]]; then
        echo -e "\e[90m${timestamp} \e[33mINFO\e[0m $1"
    else
        echo -e "${timestamp} INFO $1"
    fi
}

# Print an error message
function error() {
    local timestamp="$(date +'%Y-%m-%d %H:%M:%S')"
    if [[ -t 1 ]]; then
        echo -e "\e[90m${timestamp} \e[31mERROR\e[0m $1"
    else
        echo -e "${timestamp} ERROR $1"
    fi
}

# Calculate the top of the android source tree
TOP="${ANDROID_BUILD_TOP:-$(dirname "${BASH_SOURCE[0]}")/../../../../..}"
export ANDROID_BUILD_TOP="$TOP"

# The build server sets DIST_DIR, use a sane default for local builds
export DIST_DIR=${DIST_DIR:-$TOP/out/dist}

# Directory that holds the static patches that are included in this script (in
# contrast to the user supplied --patch-dir directory that is used to
# dynamically create or apply patches)
BUNDLED_PATCHES="$(readlink -f $(dirname "${BASH_SOURCE[0]}")/patches)"

# Add the soong and host bin directories to the PATH so that tools can be found by CI
export PATH="$TOP/build/soong/bin:$PATH"
export PATH="$TOP/${OUT_DIR:-out}/host/linux-x86/bin:$PATH"

# This is only set if running on the build server
RUNNING_ON_BUILD_SERVER=${BUILD_NUMBER:=}

# The current build ID (set if running on a build server)
BUILD_NUMBER=${BUILD_NUMBER:=local-build}

# Ensure that TARGET_PRODUCT, TARGET_RELEASE, and TARGET_BUILD_VARIANT are set.
# Sets them to default values if they are not set (sdk, sdk_finalization, and userdebug respectively).
export TARGET_PRODUCT=${TARGET_PRODUCT:-sdk}
export TARGET_RELEASE=${TARGET_RELEASE:-sdk_finalization}
export TARGET_BUILD_VARIANT=${TARGET_BUILD_VARIANT:-userdebug}

# Paths to all projects that this tool (potentially) modifies
declare -a PROJECTS
PROJECTS+=(build/release)
PROJECTS+=(build/soong)
PROJECTS+=(cts)
PROJECTS+=(development)
PROJECTS+=(frameworks/base)
PROJECTS+=(frameworks/libs/modules-utils)
PROJECTS+=(frameworks/opt/net/wifi)
PROJECTS+=(libcore)
PROJECTS+=(packages/apps/Settings)
PROJECTS+=(packages/modules/Permission)
PROJECTS+=(packages/modules/SdkExtensions)
PROJECTS+=(packages/modules/common)
PROJECTS+=(platform_testing)
PROJECTS+=(prebuilts/abi-dumps/ndk)
PROJECTS+=(prebuilts/abi-dumps/platform)
PROJECTS+=(prebuilts/sdk)
PROJECTS+=(tools/platform-compat)
PROJECTS+=(vendor/google/release)
PROJECTS+=(vendor/google_shared/build/release)
PROJECTS+=($(cd $TOP && find prebuilts/module_sdk -mindepth 1 -maxdepth 1 -type d))

for project in $(cd $BUNDLED_PATCHES && find * -type f | xargs dirname | sort -u); do
    if [[ ! " ${PROJECTS[*]} " =~ " ${project} " ]]; then
        error "$project has bundled patches but is not part of PROJECTS"
        exit 1
    fi
done

# Define the m function to run the build.
function m() {
    "$TOP/build/soong/soong_ui.bash" --make-mode \
        "TARGET_PRODUCT=$TARGET_PRODUCT" \
        "TARGET_RELEASE=$TARGET_RELEASE" \
        "TARGET_BUILD_VARIANT=$TARGET_BUILD_VARIANT" \
        "$@"
}

function git() {
    if [[ $RUNNING_ON_BUILD_SERVER ]]; then
        $(which git) \
            -c init.defaultBranch=main \
            -c user.email=buildbot@google.com \
            -c user.name=BuildBot \
            "$@"
    else
        $(which git) "$@"
    fi
}

# Create a git commit.
#
# Will create the topic $BRANCH and add all modified files in a given project
# before committing the changes.
#
# $1: project path relative to $TOP
# stdin: commit message
function git_commit() {
    local project="$1"

    pushd "$TOP/$project"
    if [[ "$(git branch --show-current)" != "$BRANCH" ]]; then
        git checkout -b "$BRANCH" goog/main
    fi
    git add .
    git commit -F -
    popd
}

# Fetch patches from a completed job on the build server
#
# $1: path to directory in which to store the patch files
# $2: build target on the build server, e.g. finalize-ndk
# $3: (optional) build server job ID; if not provided defaults to the latest
#     known good build
function download_patches_to_patchdir() {
    local patch_dir="$1"
    local build_server_target="$2"
    local build_server_id="$3"

    if [[ -z "$build_server_id" ]]; then
        build_server_id="$(/google/bin/releases/android/ab/ab.par \
            green_cl \
            --branch git_main-sdk_finalization-release \
            --target $build_server_target \
            --custom_raw_format='{o[buildId]}')"
    fi

    mkdir -p $patch_dir
    pushd "$patch_dir"
    /google/bin/releases/android/fetch_artifact/fetch_artifact.par \
        --parallelism 8 \
        --preserve_directory_structure \
        --bid $build_server_id \
        --target $build_server_target \
        'patches/**/*'
    popd
}

# Traverse the Android tree and create patch files for all commits on the topic
# $BRANCH.
#
# $1: path to directory in which to store the patch files
function format_patches_into_patchdir() {
    local patch_dir="$1"
    mkdir -p $patch_dir

    for project in "${PROJECTS[@]}"; do
        pushd "$TOP/$project"
        if [[ "$(git branch --show-current)" == "$BRANCH" ]]; then
            mkdir -p "$patch_dir/$project"
            git format-patch -o "$patch_dir/$project" "$BRANCH" ^goog/main
        fi
        popd
    done
}

# Create the topic $BRANCH and apply patches to a given project
#
# $1: project path relative to $TOP
# $2, ...: paths to patch files to apply
function apply_patches() {
    local project="$1"
    shift

    pushd "$TOP/$project"
    if [[ "$(git branch --show-current)" != "$BRANCH" ]]; then
        if [[ $RUNNING_ON_BUILD_SERVER ]]; then
            git checkout -b "$BRANCH"
        else
            git checkout -b "$BRANCH" goog/main
        fi
    fi
    git am --whitespace=nowarn $*
    if [[ ! $RUNNING_ON_BUILD_SERVER ]]; then
        # the CLs were presumably downloaded from the build server; claim
        # ownership of them to be able to upload them to gerrit
        git rebase --exec 'git commit --amend --reset-author -C HEAD' goog/main
    fi
    popd
}

# Create the topic $BRANCH and apply patches to the Android tree
#
# The patches are expected to have been created by
# format_patches_into_patchdir.
#
# $1: path to directory of patches
function apply_patches_from_patchdir() {
    local patch_dir="$1"
    if [[ ! -d "$patch_dir" ]]; then
        info "nothing to do: patch directory does not exist: $patch_dir"
        return
    fi

    if [[ $(find "$patch_dir" -type f | wc -l) -eq 0 ]]; then
        info "nothing to do: no patches found in $patch_dir"
        return
    fi

    for absolute_project_path in $(find "$patch_dir" -type f | xargs dirname | sort -u); do
        relative_project_path="${absolute_project_path#$patch_dir/}"
        # Hack to remove stray patches/ folder. NOOP if there isn't one
        project="${relative_project_path#patches/}"
        apply_patches \
            "$project" \
            $(find "$absolute_project_path" -name "*.patch" | sort)
    done
}

# Prepare the Android tree for git write operations (needed if running on the
# build server)
#
# The build servers use read-only git worktrees. Replace these with local git
# repositories to allow the creation of git commits.
function setup_build_server() {
    if [[ ! $RUNNING_ON_BUILD_SERVER ]]; then
        error "setup_build_server should only be called when running on a build server"
        exit 1
    fi

    echo "== Build server debug info start =="

    # Temporary debug info
    find --version

    local prebuilt_cached="$TOP/out/prebuilt_cached"
    if [[ -d "${prebuilt_cached}" ]]; then
      pushd ${prebuilt_cached}
      echo "These are the files passed in from previous builds:"
      find $(pwd) -type f
      popd
    else
      echo "No prebuilt_cached directory found at: ${prebuilt_cached}"
    fi

    # Might as well dump this
    env

    echo "== Build server debug info end =="

    for project in "${PROJECTS[@]}"; do
        pushd "$TOP/$project"
        rm .git # regular file when using git worktrees
        git init
        git add .
        git commit --quiet --allow-empty -m "base commit"
        git tag goog/main
        popd
    done
}

# Function to get the project directory based on the release config.
#
# $1: the release config
# @return {int} 0 on success, 1 on error.
function get_project_for_release() {
    local release_config="$1"
    local project=""

    case "$release_config" in
        "next" | "trunk")
            project="vendor/google_shared/build/release"
            ;;
        "trunk_staging")
            project="build/release"
            ;;
        "sdk_finalization")
            project="vendor/google/release"
            ;;
        *)
            echo "Error: Unexpected release config '$release_config'" >&2
            exit 1
            ;;
    esac
    echo "$project"
    return 0
}

# Set build flags for the given release config.
#
# $1: the release config
# $2, ...: KEY=VALUE pairs of build flags and the value to assign them
function set_build_flags() {
    local release_config="$1"
    shift

    local project=$(get_project_for_release "$release_config")

    build-flag --quiet --release=$release_config set --dir $project $@
    git_commit $project <<EOF
$release_config: update SDK related build flag(s)

Bug: $BUG
Test: N/A
Flag: NONE platform SDK finalization
EOF
}

# Temporarily set RELEASE_PLATFORM_PROSPECTIVE_SDK_VERSION_FULL. Caller is
# expected to call clear_prospective_sdk_version_full before exiting.
function set_prospective_sdk_version_full() {
    local value="$1"

    cat > $TOP/vendor/google_shared/build/release/flag_values/cp2a/RELEASE_PLATFORM_PROSPECTIVE_SDK_VERSION_FULL.textproto <<EOF
name: "RELEASE_PLATFORM_PROSPECTIVE_SDK_VERSION_FULL"
value: {
  string_value: "$value"
}
EOF
    trap "rm -f $TOP/vendor/google_shared/build/release/flag_values/cp2a/RELEASE_PLATFORM_PROSPECTIVE_SDK_VERSION_FULL.textproto" EXIT
}

# Unset RELEASE_PLATFORM_PROSPECTIVE_SDK_VERSION_FULL
function clear_prospective_sdk_version_full() {
    # (build-flag doesn't allow unsetting flag values, so remove it explicitly. FIXME: hard-codes cp2a as next)
    rm -f $TOP/vendor/google_shared/build/release/flag_values/cp2a/RELEASE_PLATFORM_PROSPECTIVE_SDK_VERSION_FULL.textproto
}
