# Copyright (C) 2025 The Android Open Source Project
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
#

# Large screen common
$(call inherit-product, $(SRC_TARGET_DIR)/product/large_screen_common.mk)

# Common overlay for foldables, the sequence of overlays matters as the overlays that come first will override the ones that come later.
PRODUCT_PACKAGE_OVERLAYS += $(SRC_TARGET_DIR)/product/large_screen_foldable_common/overlay

ifneq ($(RELEASE_PACKAGE_VIRTUAL_GAMEPAD),)
    # Resource overlay for the gamepad
    PRODUCT_PACKAGE_OVERLAYS += $(SRC_TARGET_DIR)/product/large_screen_foldable_common/overlay_gamepad

    # VirtualGamepad on foldables
    PRODUCT_PACKAGES += \
        VirtualGamepad
endif

