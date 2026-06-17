#!/usr/bin/env python3
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

import argparse
import sys
import time
import grpc
from google.protobuf.json_format import MessageToJson

# Import the client library and generated stubs
from soong_api_client import SoongApiClient
import soong_api_pb2_grpc

class SoongApiQuery:
    def __init__(self):
        pass

    def run(self):
        parser = argparse.ArgumentParser(
            description="Soong API Query Tool: A native AOSP tool to query build metadata.",
            formatter_class=argparse.RawDescriptionHelpFormatter,
            epilog="""
Usage Examples:
  1. Start a persistent server:
     $ soong_api_query -i

  2. Query a running server (in another terminal):
     $ soong_api_query GetModule --name libc --port <PORT_NUMBER>

  3. One-shot query (starts and stops a private server automatically):
     $ soong_api_query GetModule --name libc
            """
        )

        parser.add_argument(
            "-i", "--interactive",
            action="store_true",
            help="Keep the gRPC server running for external clients."
        )
        parser.add_argument(
            "--port",
            type=int,
            help="Connect to an existing server on this port instead of starting a new one."
        )
        parser.add_argument(
            "--rebuild",
            action=argparse.BooleanOptionalAction,
            default=True,
            help="Rebuild soong_api.db if needed."
        )
        parser.add_argument(
            "method",
            nargs="?",
            help="The gRPC method to call (e.g., GetModule)"
        )

        args, unknown = parser.parse_known_args()

        try:
            # Scenario A: Connect to an existing port (Client-only mode)
            if args.port:
                if not args.method:
                    print("Error: A method name is required when using --port.")
                    sys.exit(1)
                self._execute_remote_query(args.port, args.method, unknown)
                return

            # Scenario B: Lifecycle management mode (Starts a server)
            with SoongApiClient(rebuild=args.rebuild) as client:
                if args.interactive or not args.method:
                    self._show_server_info(client.port)
                    try:
                        while True:
                            time.sleep(1)
                    except KeyboardInterrupt:
                        print("\nStopping Soong API Server...")
                    return

                self._execute_query(client, args.method, unknown)

        except KeyboardInterrupt:
            print("\nInterrupted.")
        except Exception as e:
            print(f"Fatal Error: {e}")
            sys.exit(1)

    def _show_server_info(self, port):
        """Prints clear instructions on how to query the active server."""
        print(f"\n🚀 Soong API Server is active on localhost:{port}")
        print("=" * 70)
        print("The server is ready. To run queries, open ANOTHER terminal and run:")
        print(f"\n  soong_api_query GetModule --name <MODULE_NAME> --port {port}")
        print(f"  soong_api_query GetAllModules --port {port}")
        print("\nOr use external tools:")
        print(f"  - Postman: Create gRPC request to 'localhost:{port}'")
        print("=" * 70)
        print("Press Ctrl+C to shut down the server.")

    def _execute_remote_query(self, port, method_name, raw_params):
        """Connects to an existing port and executes a query."""
        channel = grpc.insecure_channel(f'localhost:{port}')
        stub = soong_api_pb2_grpc.SoongApiServiceStub(channel)

        # We need a dummy client object that has the stub but didn't start a server
        class RemoteClient:
            def __init__(self, stub):
                self.stub = stub
            def GetAllModules(self):
                import soong_api_pb2
                return self.stub.GetAllModules(soong_api_pb2.GetAllModulesRequest())
            def GetModule(self, name=None, **kwargs):
                import soong_api_pb2
                return self.stub.GetModule(soong_api_pb2.GetModuleRequest(name=name))
            def GetModuleByInstallPath(self, install_path=None, **kwargs):
                import soong_api_pb2
                return self.stub.GetModuleByInstallPath(soong_api_pb2.GetModuleByInstallPathRequest(install_path=install_path))

        remote_client = RemoteClient(stub)
        self._execute_query(remote_client, method_name, raw_params)

    def _execute_query(self, client, method_name, raw_params):
        """Common logic to execute a method and print JSON."""
        params = {}
        for i in range(0, len(raw_params), 2):
            key = raw_params[i].lstrip('-')
            if i + 1 < len(raw_params):
                params[key] = raw_params[i+1]

        if not hasattr(client, method_name):
            print(f"Error: Unknown method '{method_name}'.")
            sys.exit(1)

        try:
            method = getattr(client, method_name)
            responses = method(**params)
            for msg in responses:
                print(MessageToJson(msg, preserving_proto_field_name=True))
        except Exception as e:
            print(f"Error during gRPC call: {e}")

if __name__ == "__main__":
    tool = SoongApiQuery()
    tool.run()
