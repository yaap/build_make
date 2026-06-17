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

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import org.junit.Before;
import org.junit.Rule;
import org.junit.Test;
import org.junit.rules.TemporaryFolder;
import org.junit.runner.RunWith;
import org.junit.runners.JUnit4;

import java.io.File;
import java.io.FileOutputStream;
import java.io.FileWriter;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.ResultSet;
import java.sql.Statement;
import java.util.HashMap;
import java.util.Map;
import java.util.zip.ZipEntry;
import java.util.zip.ZipOutputStream;

/**
 * Tests for {@link SoongApiLoader}.
 */
@RunWith(JUnit4.class)
public class SoongApiLoaderTest {

    @Rule
    public TemporaryFolder tempFolder = new TemporaryFolder();

    private File outputDb;
    private SoongApiLoader loader;

    // Mock Soong JSON content
    private final String jsonContentSoong = "[\n" +
            "  {\n" +
            "    \"name\": \"moduleSoong\", \n" +
            "    \"type\": \"java_library\", \n" +
            "    \"path\": \"frameworks/base\", \n" +
            "    \"install_files\": [\"/system/lib/a.jar\"]\n" +
            "  }\n" +
            "]";

    // Mock Make JSON content
    private final String jsonContentMake = "[\n" +
            "  {\n" +
            "    \"name\": \"moduleMake\", \n" +
            "    \"type\": \"cc_binary\", \n" +
            "    \"path\": \"vendor/foo\", \n" +
            "    \"install_files\": [\"/vendor/bin/foo\"]\n" +
            "  }\n" +
            "]";

    @Before
    public void setUp() throws Exception {
        loader = new SoongApiLoader();
        outputDb = tempFolder.newFile("test_soong_api.db");
    }

    @Test
    public void testLoad_fromZip_convertsJsonToSqlite() throws Exception {
        // 1. Arrange: Create a standard soong_api.zip
        File inputZip = tempFolder.newFile("soong_api.zip");
        Map<String, String> files = new HashMap<>();
        files.put("soong_api.json", jsonContentSoong);
        createMockZipFile(inputZip, files);

        // 2. Act
        loader.load(inputZip, outputDb);

        // 3. Assert
        assertModuleCount(1);
        assertModuleExists("moduleSoong", "java_library");
    }

    @Test
    public void testLoad_fromZip_loadsMultipleJsonFiles() throws Exception {
        // 1. Arrange: Create a zip with multiple JSONs and non-JSON files
        File inputZip = tempFolder.newFile("mixed_content.zip");
        Map<String, String> files = new HashMap<>();
        files.put("soong_api.json", jsonContentSoong);
        files.put("make-metadata.json", jsonContentMake);
        files.put("README.txt", "This file should be ignored.");
        files.put("folder/ignore.me", "Directories should be ignored too.");
        createMockZipFile(inputZip, files);

        // 2. Act
        loader.load(inputZip, outputDb);

        // 3. Assert: Verify both modules (Soong + Make) are present
        assertModuleCount(2);
        assertModuleExists("moduleSoong", "java_library");
        assertModuleExists("moduleMake", "cc_binary");
    }

    @Test
    public void testLoad_fromZip_noJsonFiles_throwsException() throws Exception {
        // 1. Arrange: Create a zip with no JSON files.
        // Include a directory ending in .json to verify that directories are correctly ignored.
        File inputZip = tempFolder.newFile("no_json.zip");
        Map<String, String> files = new HashMap<>();
        files.put("README.txt", "Just a text file");
        files.put("fake_dir.json/", ""); // Directory ending in .json
        createMockZipFile(inputZip, files);

        // 2. Act & Assert: Verify that an IOException is thrown with the correct message.
        IOException exception = org.junit.Assert.assertThrows(
                IOException.class,
                () -> loader.load(inputZip, outputDb)
        );

        assertTrue("Exception message should indicate no .json files were found",
                exception.getMessage().contains("No .json files found in no_json.zip"));
    }

    @Test
    public void testLoad_fromJson_convertsJsonToSqlite() throws Exception {
        // 1. Arrange: Create a standalone json file
        File inputJson = tempFolder.newFile("soong_api.json");
        createMockJsonFile(inputJson, jsonContentSoong);

        // 2. Act
        loader.load(inputJson, outputDb);

        // 3. Assert
        assertModuleCount(1);
        assertModuleExists("moduleSoong", "java_library");
    }

    // --- Helper Methods ---

    private void createMockZipFile(File zipFile, Map<String, String> fileEntries) throws Exception {
        try (FileOutputStream fos = new FileOutputStream(zipFile);
             ZipOutputStream zos = new ZipOutputStream(fos)) {

            for (Map.Entry<String, String> entry : fileEntries.entrySet()) {
                ZipEntry zipEntry = new ZipEntry(entry.getKey());
                zos.putNextEntry(zipEntry);
                zos.write(entry.getValue().getBytes(StandardCharsets.UTF_8));
                zos.closeEntry();
            }
        }
    }

    private void createMockJsonFile(File jsonFile, String content) throws Exception {
        try (FileWriter writer = new FileWriter(jsonFile, StandardCharsets.UTF_8)) {
            writer.write(content);
        }
    }

    private void assertModuleCount(int expectedCount) throws Exception {
        assertTrue("Output DB should exist", outputDb.exists());
        String url = "jdbc:sqlite:" + outputDb.getAbsolutePath();
        try (Connection conn = DriverManager.getConnection(url);
             Statement stmt = conn.createStatement()) {

            ResultSet rs = stmt.executeQuery("SELECT count(*) FROM modules");
            assertTrue(rs.next());
            assertEquals("Module count mismatch", expectedCount, rs.getInt(1));
        }
    }

    private void assertModuleExists(String moduleName, String expectedType) throws Exception {
        String url = "jdbc:sqlite:" + outputDb.getAbsolutePath();
        try (Connection conn = DriverManager.getConnection(url);
             Statement stmt = conn.createStatement()) {

            ResultSet rs = stmt.executeQuery("SELECT metadata FROM modules WHERE name = '" + moduleName + "'");
            assertTrue("Module '" + moduleName + "' should exist", rs.next());

            String jsonMetadata = rs.getString("metadata");
            assertTrue("Metadata should contain type: " + expectedType,
                    jsonMetadata.contains("\"type\":\"" + expectedType + "\""));

            // Validate snake_case convention
            assertTrue("JSON must use snake_case 'install_files'",
                    jsonMetadata.contains("\"install_files\""));
        }
    }
}