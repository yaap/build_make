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

# The major part of the next $MAJOR.$MINOR SDK version to finalize
MAJOR=37

# The minor part of the next $MAJOR.$MINOR SDK version to finalize
MINOR=0

# The SDK extension version to finalize (as part of the platform)
SDK_EXT_VERSION=22

# The next Android "marketing version", e.g. Android 16 for Baklava
MARKETING_VERSION=17

# The bug used to track the finalization
BUG=480974361

# The topic branch to perform the work on
BRANCH="finalize-$MAJOR.$MINOR"
