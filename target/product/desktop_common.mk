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
#

# Contains standard elements for Android desktop devices.

# Should generally be inherited first as using an HSUM configuration can affect downstream choices
# (such as ensuring that the HSUM-variants of packages are selected).

$(call inherit-product, build/make/target/product/hsu_as_login.mk)

PRODUCT_PACKAGES += \
    DesktopCommonConfigOverlay \
    preinstalled-packages-desktop-common.xml \
    # TODO(482063300): Remove wificond once desktop migrates off of it \
    wificond \
