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

package com.android.soong.api.loader;

import com.android.soong.api.proto.Module;
import com.google.gson.Gson;
import com.google.gson.JsonObject;
import com.google.gson.stream.JsonReader;
import com.google.protobuf.util.JsonFormat;

import java.io.*;
import java.nio.charset.StandardCharsets;
import java.sql.*;
import java.util.Enumeration;
import java.util.zip.ZipEntry;
import java.util.zip.ZipFile;

/**
 * SoongApiLoader handles the ingestion of Soong metadata (JSON/ZIP) into a SQLite database.
 */
public class SoongApiLoader {

    /**
     * Executes the conversion logic: reads the input file and populates the SQLite database.
     *
     * @param inputFile The input soong_api.zip or soong_api.json file.
     * @param outputDb The output SQLite DB file.
     */
    public void load(File inputFile, File outputDb) throws IOException, SQLException {
        if (!inputFile.exists()) {
            throw new FileNotFoundException("Input file not found: " + inputFile.getAbsolutePath());
        }

        // Establish JDBC connection
        String url = "jdbc:sqlite:" + outputDb.getAbsolutePath();
        try (Connection conn = DriverManager.getConnection(url)) {
            conn.setAutoCommit(false); // Enable transaction for performance

            // Initialize Schema
            initializeDb(conn);

            String inputFileName = inputFile.getName();
            if (inputFileName.endsWith(".zip")) {
                processZip(conn, inputFile);
            } else if (inputFileName.endsWith(".json")) {
                processJson(conn, inputFile);
            } else {
                throw new IllegalArgumentException("Input file must be a .zip or .json file: " + inputFileName);
            }

            conn.commit();
        }
    }

    private void initializeDb(Connection conn) throws SQLException {
        try (Statement stmt = conn.createStatement()) {
            stmt.execute("DROP TABLE IF EXISTS modules");
            // metadata column stores the programmatic JSON representation of the Module proto.
            stmt.execute("CREATE TABLE modules (name TEXT, metadata TEXT)");
            stmt.execute("CREATE INDEX idx_name ON modules(name)");
        }
    }

    private void processZip(Connection conn, File inputZip) throws IOException, SQLException {
        try (ZipFile zf = new ZipFile(inputZip)) {
            Enumeration<? extends ZipEntry> entries = zf.entries();
            boolean hasJsonFiles = false;

            while (entries.hasMoreElements()) {
                ZipEntry entry = entries.nextElement();

                if (!entry.isDirectory() && entry.getName().toLowerCase().endsWith(".json")) {
                    hasJsonFiles = true;

                    try (InputStream is = zf.getInputStream(entry);
                         InputStreamReader isr = new InputStreamReader(is, StandardCharsets.UTF_8)) {
                        parseAndInsertMetadata(conn, isr);
                    }
                }
            }

            if (!hasJsonFiles) {
                throw new IOException("No .json files found in " + inputZip.getName());
            }
        }
    }

    private void processJson(Connection conn, File inputJson) throws IOException, SQLException {
        try (FileInputStream fis = new FileInputStream(inputJson);
             InputStreamReader isr = new InputStreamReader(fis, StandardCharsets.UTF_8)) {
            parseAndInsertMetadata(conn, isr);
        }
    }

    private void parseAndInsertMetadata(Connection conn, InputStreamReader isr) throws IOException, SQLException {
        Gson gson = new Gson();
        JsonFormat.Parser parser = JsonFormat.parser().ignoringUnknownFields();
        JsonFormat.Printer printer = JsonFormat.printer()
            .preservingProtoFieldNames() // Preserve `install_files` instead of `installFiles`
            .omittingInsignificantWhitespace();

        String sql = "INSERT INTO modules (name, metadata) VALUES (?, ?)";

        try (JsonReader reader = new JsonReader(isr);
             PreparedStatement pstmt = conn.prepareStatement(sql)) {

            reader.beginArray(); // Start reading the JSON array [

            while (reader.hasNext()) {
                // Stream reading to minimize memory footprint.
                JsonObject jsonObject = gson.fromJson(reader, JsonObject.class);
                String rawJson = gson.toJson(jsonObject);

                // Programmatic conversion from JSON to Protobuf for validation and access.
                Module.Builder moduleBuilder = Module.newBuilder();
                parser.merge(rawJson, moduleBuilder);
                Module module = moduleBuilder.build();

                String name = module.getName();
                // Skip modules that do not have a name.
                if (name.isEmpty()) {
                    System.err.println("Warning: Skipping module with no name: " + rawJson);
                    continue;
                }
                // Store the sanitized JSON string from Protobuf.
                String protoJson = printer.print(module);

                pstmt.setString(1, name);
                pstmt.setString(2, protoJson);
                pstmt.addBatch();
            }

            reader.endArray(); // End reading the JSON array ]
            pstmt.executeBatch();
        }
    }
}
