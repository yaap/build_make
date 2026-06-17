#
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

# Contains standard elements for devices running in Headless System User Mode
# in which the Headless System User (user 0) acts as a login screen.

# Should generally be inherited first as using an HSUM configuration can affect downstream choices
# (such as ensuring that the HSUM-variants of packages are selected).

$(call inherit-product, build/make/target/product/hsum_common.mk)

PRODUCT_PACKAGES += \
    HsuAsLoginConfigOverlay \
    preinstalled-packages-platform-hsu-as-login.xml

