/*
 * Copyright (C) 2026 The Android Open Source Project
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package com.android.soong.api.client;

import com.android.soong.api.proto.GetAllModulesRequest;
import com.android.soong.api.proto.GetModuleByInstallPathRequest;
import com.android.soong.api.proto.GetModuleRequest;
import com.android.soong.api.proto.Module;
import com.android.soong.api.proto.SoongApiServiceGrpc;
import io.grpc.ManagedChannel;
import io.grpc.ManagedChannelBuilder;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.Iterator;
import java.util.concurrent.TimeUnit;

/**
 * A Java Client for the Soong API gRPC Service.
 *
 * <p>This client handles the lifecycle of the local gRPC server automatically.
 * On initialization, it checks for the existence of {@code soong_api.db}. By default,
 * it triggers a build using the Android build system to ensure data freshness.
 * The server is started on an ephemeral port, and the port is communicated
 * back to the client via a token-based temporary file.
 *
 * <p>Implements {@link AutoCloseable} to ensure the server process is terminated
 * when the client is closed.
 */
public class SoongApiClient implements AutoCloseable {

    private static final String SOONG_UI_BASH_REL = "build/soong/soong_ui.bash";
    // Default relative paths if environment variables are not set.
    private static final String DEFAULT_HOST_OUT_REL = "out/host/linux-x86";
    private static final String DEFAULT_OUT_DIR_REL = "out";

    private final Path androidTop;
    private final Path serverBinary;
    private final Path databasePath;
    private final boolean isCustomDb;

    private Process serverProcess;
    private ManagedChannel channel;
    private SoongApiServiceGrpc.SoongApiServiceBlockingStub blockingStub;
    private int port;

    /**
     * Standard constructor using environment build paths and enabling rebuild by default.
     */
    public SoongApiClient() throws IOException, InterruptedException {
        this(null, true);
    }

    /**
     * Constructor allowing custom database path and rebuild control.
     *
     * @param customDbPath Path to an existing soong_api.db. If provided, build logic is bypassed.
     * @param rebuild Whether to trigger 'm soong_api.db' when using standard paths.
     */
    public SoongApiClient(String customDbPath, boolean rebuild) throws IOException, InterruptedException {
        checkOs();
        this.androidTop = findAndroidTop();
        this.serverBinary = resolveServerBinary();

        if (customDbPath != null) {
            this.databasePath = Paths.get(customDbPath);
            this.isCustomDb = true;
        } else {
            this.databasePath = resolveDatabasePath();
            this.isCustomDb = false;
        }

        ensureBuildArtifacts(rebuild);
        startServer();
    }

    /**
     * Returns the port number the local server is listening on.
     */
    public int getPort() {
        return this.port;
    }

    // --- Public API Methods ---

    /**
     * Returns a stream of all modules defined in the build system.
     */
    public Iterator<Module> getAllModules() {
        return blockingStub.getAllModules(GetAllModulesRequest.getDefaultInstance());
    }

    /**
     * Convenience method to get modules by name, hiding the Request builder.
     *
     * @param name The name of the module (e.g., "libc", "Settings").
     * @return A stream of modules matching the name.
     */
    public Iterator<Module> getModule(String name) {
        GetModuleRequest request = GetModuleRequest.newBuilder().setName(name).build();
        return blockingStub.getModule(request);
    }

    /**
     * Convenience method to get modules by install path, hiding the Request builder.
     *
     * @param installPath The absolute installation path (e.g., "/system/lib64/libc.so").
     * @return A stream of modules contributing to that path.
     */
    public Iterator<Module> getModuleByInstallPath(String installPath) {
        GetModuleByInstallPathRequest request =
                GetModuleByInstallPathRequest.newBuilder().setInstallPath(installPath).build();
        return blockingStub.getModuleByInstallPath(request);
    }

    // --- Lifecycle and Internal Management Methods ---

    private void checkOs() {
        String os = System.getProperty("os.name").toLowerCase();
        if (!os.contains("linux")) {
            throw new UnsupportedOperationException("SoongApiClient currently supports Linux only.");
        }
    }

    private Path findAndroidTop() throws IOException {
        String envTop = System.getenv("ANDROID_BUILD_TOP");
        if (envTop != null && !envTop.isEmpty()) {
            return Paths.get(envTop);
        }
        Path cwd = Paths.get(".").toAbsolutePath().normalize();
        if (Files.exists(cwd.resolve(SOONG_UI_BASH_REL))) {
            return cwd;
        }
        throw new IOException("Could not locate Android Root. Please set ANDROID_BUILD_TOP.");
    }

    /**
     * Resolves the server binary path, respecting ANDROID_HOST_OUT if set.
     */
    private Path resolveServerBinary() {
        String hostOut = System.getenv("ANDROID_HOST_OUT");
        if (hostOut != null && !hostOut.isEmpty()) {
            return Paths.get(hostOut).resolve("bin/soong_api_server");
        }
        return androidTop.resolve(DEFAULT_HOST_OUT_REL).resolve("bin/soong_api_server");
    }

    /**
     * Resolves the database path, respecting OUT_DIR if set.
     */
    private Path resolveDatabasePath() {
        String outDir = System.getenv("OUT_DIR");
        Path baseOut;
        if (outDir != null && !outDir.isEmpty()) {
            baseOut = Paths.get(outDir);
            if (!baseOut.isAbsolute()) {
                baseOut = androidTop.resolve(baseOut);
            }
        } else {
            baseOut = androidTop.resolve(DEFAULT_OUT_DIR_REL);
        }
        String product = System.getenv("TARGET_PRODUCT");
        if (product == null || product.isEmpty()) {
            product = "generic";
        }
        return baseOut.resolve("soong/soong_api").resolve(product).resolve("soong_api.db");
    }

    private void ensureBuildArtifacts(boolean rebuild) throws IOException, InterruptedException {
        if (isCustomDb) {
            if (!Files.exists(databasePath)) {
                throw new IOException("Custom database file not found: " + databasePath);
            }
            return;
        }

        boolean artifactsExist = Files.exists(serverBinary) && Files.exists(databasePath);
        String targetProduct = System.getenv("TARGET_PRODUCT");

        // Ideal Case: Lunch exists, ensure freshness
        if (targetProduct != null && !targetProduct.isEmpty()) {
            if (rebuild || !artifactsExist) {
                System.out.println("Ensuring Soong API artifacts are up-to-date...");
                runBuild();
            }
            return;
        }

        // Fallback Case: No lunch, use existing if available
        if (artifactsExist) {
            System.out.println("Warning: Build environment not detected. Using existing database.");
            return;
        }

        throw new IOException("Soong API artifacts missing and 'lunch' environment not set.");
    }

    private void runBuild() throws IOException, InterruptedException {
        ProcessBuilder pb = new ProcessBuilder(
                androidTop.resolve(SOONG_UI_BASH_REL).toString(),
                "--make-mode",
                "soong_api.db",
                "soong_api_server");

        pb.directory(androidTop.toFile());
        pb.inheritIO();
        int exitCode = pb.start().waitFor();
        if (exitCode != 0) {
            throw new IOException("Build failed with exit code: " + exitCode);
        }
    }

    private void startServer() throws IOException {
        String timestamp = String.valueOf(System.currentTimeMillis());
        System.out.println("Starting soong_api_server...");

        ProcessBuilder pb = new ProcessBuilder(
                serverBinary.toString(),
                "--db_path", databasePath.toString(),
                "--timestamp", timestamp);

        pb.redirectErrorStream(true);
        serverProcess = pb.start();

        // Wait for the server to write its bound port to the temporary file
        this.port = waitForPortFile(timestamp);
        System.out.println("Server detected on port " + port + ".");
        createChannel(port);
    }

    private int waitForPortFile(String token) throws IOException {
        String tmpDir = System.getProperty("java.io.tmpdir");
        Path portFile = Paths.get(tmpDir).resolve("SoongApiServer-" + token + ".txt");
        long start = System.currentTimeMillis();
        long timeoutMs = 10000; // 10 seconds

        while (System.currentTimeMillis() - start < timeoutMs) {
            if (!serverProcess.isAlive()) {
                throw new IOException("Server process died unexpectedly during startup.");
            }
            if (Files.exists(portFile)) {
                try {
                    String content = Files.readString(portFile).trim();
                    if (!content.isEmpty()) {
                        return Integer.parseInt(content);
                    }
                } catch (NumberFormatException ignored) {
                    // File might be in the middle of being written
                }
            }
            try {
                TimeUnit.MILLISECONDS.sleep(100);
            } catch (InterruptedException e) {
                Thread.currentThread().interrupt();
                throw new IOException("Interrupted while waiting for server port file.", e);
            }
        }
        throw new IOException("Timed out waiting for port file: " + portFile);
    }

    private void createChannel(int port) {
        this.channel = ManagedChannelBuilder.forAddress("localhost", port)
                .usePlaintext()
                .build();
        this.blockingStub = SoongApiServiceGrpc.newBlockingStub(channel);
    }

    @Override
    public void close() {
        if (channel != null) {
            channel.shutdown();
            try {
                if (!channel.awaitTermination(2, TimeUnit.SECONDS)) {
                    channel.shutdownNow();
                }
            } catch (InterruptedException e) {
                channel.shutdownNow();
            }
        }

        if (serverProcess != null) {
            System.out.println("Shutting down soong_api_server...");
            serverProcess.destroy();
            try {
                if (!serverProcess.waitFor(2, TimeUnit.SECONDS)) {
                    serverProcess.destroyForcibly();
                }
            } catch (InterruptedException e) {
                serverProcess.destroyForcibly();
            }
        }
    }
}
