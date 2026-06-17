/*
 * Copyright (C) 2025 The Android Open Source Project
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

package com.android.soong.api.server;

import com.android.soong.api.db.SoongApiDao;
import io.grpc.protobuf.services.ProtoReflectionService;
import io.grpc.Server;
import io.grpc.ServerBuilder;

import java.io.File;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.SQLException;
import java.util.concurrent.TimeUnit;

/**
 * Main entry point for the Soong API Server (SNAPI).
 * <p>
 * Usage: {@code soong_api_server --db_path <path> [--port <port>] [--timestamp <token>]}
 */
public class SoongApiServer {
    private Server server;
    private Connection connection;
    private Path portFilePath;

    private void start(int port, String dbPath, String timestamp) throws IOException, SQLException {
        // 1. Initialize DB Connection
        String url = "jdbc:sqlite:" + dbPath;
        connection = DriverManager.getConnection(url);
        System.out.println("Connected to Soong API database: " + dbPath);

        // 2. Initialize DAO and Service
        SoongApiDao dao = new SoongApiDao(connection);
        SoongApiServiceImpl service = new SoongApiServiceImpl(dao);

        // 3. Build and Start gRPC Server
        // Port 0 (default) allows the system to assign a random free ephemeral port
        server = ServerBuilder.forPort(port)
                .addService(service)
                .addService(ProtoReflectionService.newInstance())
                .build()
                .start();

        int actualPort = server.getPort();
        System.out.println("Soong API Server started, listening on " + actualPort);

        // 4. Write the actual port to a temp file for the client to discover
        writePortToFile(actualPort, timestamp);

        // 5. Add Shutdown Hook for graceful termination
        Runtime.getRuntime().addShutdownHook(new Thread(() -> {
            System.err.println("*** shutting down SNAPI server since JVM is shutting down");
            try {
                SoongApiServer.this.stop();
            } catch (InterruptedException e) {
                e.printStackTrace(System.err);
            }
            System.err.println("*** SNAPI server shut down");
        }));
    }

    private void writePortToFile(int port, String timestamp) throws IOException {
        // Use timestamp/token provided by client to avoid PID collision issues
        String fileName = (timestamp != null) ? "SoongApiServer-" + timestamp + ".txt" :
                "SoongApiServer-" + ProcessHandle.current().pid() + ".txt";

        // Use the system default temporary directory instead of hardcoded /tmp
        String tmpDir = System.getProperty("java.io.tmpdir");
        this.portFilePath = Path.of(tmpDir, fileName);

        Files.writeString(this.portFilePath, String.valueOf(port));
        System.out.println("Port information written to: " + portFilePath);
    }

    private void stop() throws InterruptedException {
        if (server != null) {
            server.shutdown().awaitTermination(30, TimeUnit.SECONDS);
        }

        // Clean up the port file
        if (portFilePath != null) {
            try {
                Files.deleteIfExists(portFilePath);
            } catch (IOException e) {
                System.err.println("Failed to delete port file: " + e.getMessage());
            }
        }

        if (connection != null) {
            try {
                connection.close();
                System.out.println("Database connection closed.");
            } catch (SQLException e) {
                e.printStackTrace();
            }
        }
    }

    private void blockUntilShutdown() throws InterruptedException {
        if (server != null) {
            server.awaitTermination();
        }
    }

    public static void main(String[] args) throws IOException, InterruptedException, SQLException {
        int port = 0; // Default to 0 to enable ephemeral port assignment
        String dbPath = null;
        String timestamp = null;

        for (int i = 0; i < args.length; i++) {
            if ("--db_path".equals(args[i]) && i + 1 < args.length) {
                dbPath = args[i + 1];
                i++;
            } else if ("--port".equals(args[i]) && i + 1 < args.length) {
                port = Integer.parseInt(args[i + 1]);
                i++;
            } else if ("--timestamp".equals(args[i]) && i + 1 < args.length) {
                timestamp = args[i + 1];
                i++;
            }
        }

        if (dbPath == null) {
            System.err.println("Usage: soong_api_server --db_path <metadata.db> [--port <port>] [--timestamp <token>]");
            System.exit(1);
        }

        if (!new File(dbPath).exists()) {
            System.err.println("Error: Database file not found: " + dbPath);
            System.exit(1);
        }

        final SoongApiServer server = new SoongApiServer();
        try {
            server.start(port, dbPath, timestamp);
            server.blockUntilShutdown();
        } catch (Exception e) {
            System.err.println("SNAPI Server failed to start: " + e.getMessage());
            e.printStackTrace();
            System.exit(1);
        }
    }
}