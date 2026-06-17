#!/usr/bin/env python3
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

import sys
from google.protobuf import json_format
from soong_api_client import SoongApiClient

def print_module(module):
    """Helper to print a module in JSON format preserving field names."""
    try:
        # preserving_proto_field_name=True ensures 'install_files' matches DB/Evans output
        json_str = json_format.MessageToJson(
            module,
            preserving_proto_field_name=True,
            indent=2
        )
        print(json_str)
    except Exception as e:
        print(f"Error printing module: {e}", file=sys.stderr)

def main():
    print("Initializing Soong API Client...")

    try:
        with SoongApiClient() as client:

            # ==========================================
            # Example 1: Query by Module Name
            # ==========================================
            query_name = "Videos"
            if len(sys.argv) > 1:
                query_name = sys.argv[1]

            print(f"\n[1] Querying GetModule (Name: {query_name})")
            print("-" * 50)

            # Use the convenience method
            modules = client.GetModule(query_name)
            for mod in modules:
                print_module(mod)

            # ==========================================
            # Example 2: Query by Install Path
            # ==========================================
            install_path = "out/target/product/vsoc_x86_64/system_other/product/app/Videos/oat/x86_64/Videos.odex"

            print(f"\n[2] Querying GetModuleByInstallPath (Path: {install_path})")
            print("-" * 50)

            # Use the convenience method
            modules = client.GetModuleByInstallPath(install_path)
            for mod in modules:
                print_module(mod)

            print("-" * 50)
            print("Done.")

    except Exception as e:
        print(f"Fatal Error: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()
