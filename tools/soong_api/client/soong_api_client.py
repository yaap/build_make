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

import grpc
import os
import platform
import subprocess
import tempfile
import time
from pathlib import Path

# Import generated protobuf code
# These come from the genrule :soong_api_python_protoc_gen
import soong_api_pb2
import soong_api_pb2_grpc

class SoongApiClient:
    """
    A Python Client for the Soong API gRPC Service.

    This client handles the lifecycle of the local gRPC server automatically.
    The server is started on an ephemeral port (assigned by the OS), and the
    port is communicated back via a temporary file.
    """

    SOONG_UI_BASH_REL = "build/soong/soong_ui.bash"
    DEFAULT_OUT_DIR = "out"
    DEFAULT_HOST_OUT_REL = "out/host/linux-x86"

    def __init__(self, db_path=None, rebuild=True):
        """
        Initializes the client.
        Args:
            db_path (str, optional): Path to a custom soong_api.db file.
                If provided, build automation is bypassed.
            rebuild (bool): Whether to trigger 'm soong_api.db' when using
                the standard path. Defaults to True.
        """
        self._check_os()
        self.android_top = self._find_android_top()
        self.soong_ui = self.android_top / self.SOONG_UI_BASH_REL
        self.server_path = self._resolve_server_path()

        # Handle custom vs standard database path
        if db_path:
            self.db_path = Path(db_path)
            self._is_custom_db = True
        else:
            self.db_path = self._resolve_db_path()
            self._is_custom_db = False

        self.server_process = None
        self.channel = None
        self.stub = None
        self._port = None

        self._ensure_build_artifacts(rebuild)
        self._start_server()

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_value, traceback):
        self.close()

    @property
    def port(self):
        """Returns the port number the local server is listening on."""
        return self._port

    def close(self):
        if self.channel:
            self.channel.close()

        if self.server_process:
            print("Shutting down soong_api_server...")
            self.server_process.terminate()
            try:
                self.server_process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                self.server_process.kill()

    # --- Public API Methods ---

    def GetAllModules(self):
        """Returns an iterator of all modules defined in the build system."""
        request = soong_api_pb2.GetAllModulesRequest()
        return self.stub.GetAllModules(request)

    def GetModule(self, name=None, **kwargs):
        """
        Query modules by name.
        Args:
            name (str): The name of the module (e.g., "libc").
        """
        if not name:
            raise ValueError("The '--name' argument is required for GetModule.")
        request = soong_api_pb2.GetModuleRequest(name=name)
        return self.stub.GetModule(request)

    def GetModuleByInstallPath(self, install_path=None, **kwargs):
        """
        Query modules by their installation path.
        Args:
            install_path (str): The absolute installation path (e.g., "/system/lib64/libc.so").
        """
        if not install_path:
            raise ValueError("The '--install_path' argument is required for GetModuleByInstallPath.")
        request = soong_api_pb2.GetModuleByInstallPathRequest(install_path=install_path)
        return self.stub.GetModuleByInstallPath(request)

    # --- Internal Methods ---

    def _check_os(self):
        if platform.system() != 'Linux':
            raise OSError("SoongApiClient currently supports Linux only.")

    def _find_android_top(self):
        env_top = os.environ.get('ANDROID_BUILD_TOP')
        if env_top:
            return Path(env_top)

        # Fallback: Check CWD
        cwd = Path.cwd()
        if (cwd / self.SOONG_UI_BASH_REL).exists():
            return cwd

        raise FileNotFoundError(
            "Could not locate Android Root. Please set ANDROID_BUILD_TOP "
            "or run from the root of the source tree."
        )

    def _resolve_server_path(self):
        # Use ANDROID_HOST_OUT if available (e.g. out/host/linux-x86)
        host_out = os.environ.get('ANDROID_HOST_OUT')
        if host_out:
            return Path(host_out) / "bin/soong_api_server"

        return self.android_top / self.DEFAULT_HOST_OUT_REL / "bin/soong_api_server"

    def _resolve_db_path(self):
        # Use OUT_DIR if available, otherwise default to "out"
        out_dir = os.environ.get('OUT_DIR', self.DEFAULT_OUT_DIR)
        out_path = Path(out_dir)

        # Handle relative OUT_DIR (relative to top)
        if not out_path.is_absolute():
            out_path = self.android_top / out_path

        product = os.environ.get('TARGET_PRODUCT', 'generic')
        return out_path / "soong" / "soong_api" / product / "soong_api.db"

    def _ensure_build_artifacts(self, rebuild):
        """Follows the Decision Logic for custom vs. standard DB paths."""
        # Case 1: Custom DB Path - Bypass build logic and only check existence
        if self._is_custom_db:
            if not self.db_path.exists():
                raise FileNotFoundError(f"Custom database file not found: {self.db_path}")
            return

        # Case 2: Standard DB Path - Freshness-first, Existence-fallback logic
        artifacts_exist = self.server_path.exists() and self.db_path.exists()
        has_lunch = os.environ.get('TARGET_PRODUCT') is not None

        if has_lunch:
            if rebuild or not artifacts_exist:
                print("Ensuring Soong API artifacts are up-to-date...")
                self._run_build()
            return

        if artifacts_exist:
            print("Warning: Build environment not detected. Using existing soong_api.db (may be stale).")
            return

        raise EnvironmentError(
            "Soong API artifacts are missing and build environment (lunch) is not set. "
            "Please run 'lunch' first or ensure soong_api.db exists."
        )

    def _run_build(self):
        cmd = [str(self.soong_ui), "--make-mode", "soong_api.db", "soong_api_server"]
        try:
            # Redirect output to inherit to show build progress
            subprocess.check_call(cmd, cwd=self.android_top)
            print("Build successful.")
        except subprocess.CalledProcessError as e:
            raise RuntimeError(f"Build failed with exit code: {e.returncode}") from e

    def _start_server(self):
        # Generate a unique timestamp token to avoid file collisions
        timestamp_token = str(int(time.time() * 1000))

        print(f"Starting soong_api_server...")

        cmd = [
            str(self.server_path),
            "--db_path", str(self.db_path),
            "--timestamp", timestamp_token
        ]
        self.server_process = subprocess.Popen(
            cmd,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.STDOUT
        )

        self._port = self._wait_for_port_file(timestamp_token)
        print(f"Server detected on port {self._port}.")
        self._create_channel(self._port)

    def _wait_for_port_file(self, timestamp_token):
        """Waits for the server to create a file containing the bound port."""
        # Use system default temp directory to match server's behavior
        tmp_dir = tempfile.gettempdir()
        port_file = Path(tmp_dir) / f"SoongApiServer-{timestamp_token}.txt"

        start_time = time.time()
        timeout = 10.0 # seconds

        while time.time() - start_time < timeout:
            if self.server_process.poll() is not None:
                raise RuntimeError("Server process exited prematurely.")

            if port_file.exists():
                try:
                    content = port_file.read_text().strip()
                    if content:
                        return int(content)
                except (ValueError, OSError):
                    # File might be partially written or locked
                    pass

            time.sleep(0.1)

        raise TimeoutError(f"Timed out waiting for port file: {port_file}")

    def _create_channel(self, port):
        self.channel = grpc.insecure_channel(f'localhost:{port}')
        self.stub = soong_api_pb2_grpc.SoongApiServiceStub(self.channel)
