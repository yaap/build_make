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

package com.android.soong.api.db;

import static org.junit.Assert.*;

import com.android.soong.api.proto.Module;
import org.junit.After;
import org.junit.Before;
import org.junit.Test;
import org.junit.runner.RunWith;
import org.junit.runners.JUnit4;

import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.Statement;
import java.util.List;

/**
 * Tests for {@link SoongApiDao}.
 */
@RunWith(JUnit4.class)
public class SoongApiDaoTest {

    private Connection connection;
    private SoongApiDao dao;

    @Before
    public void setUp() throws Exception {
        connection = DriverManager.getConnection("jdbc:sqlite::memory:");

        try (Statement stmt = connection.createStatement()) {
            stmt.execute("CREATE TABLE modules (name TEXT, metadata TEXT)");
            stmt.execute("CREATE INDEX idx_name ON modules(name)");

            // --- Dataset Setup ---

            // 1. libfoo: 2 Variants (Same Path) -> Should merge into 1 Module
            insertModule(stmt, "libfoo",
                "{ \"name\": \"libfoo\", \"path\": \"src/foo\", \"install_files\": [\"/lib64/libfoo.so\"] }");
            insertModule(stmt, "libfoo",
                "{ \"name\": \"libfoo\", \"path\": \"src/foo\", \"install_files\": [\"/lib/libfoo.so\"] }");

            // 2. Videos: 2 Distinct Definitions (Different Path) -> Should keep as 2 Modules
            insertModule(stmt, "Videos",
                "{ \"name\": \"Videos\", \"path\": \"vendor/mobile\", \"install_files\": [\"/app/Videos.apk\"] }");
            insertModule(stmt, "Videos",
                "{ \"name\": \"Videos\", \"path\": \"vendor/old\", \"install_files\": [] }");

            // 3. Simple Module
            insertModule(stmt, "bar",
                "{ \"name\": \"bar\", \"path\": \"src/bar\", \"install_files\": [] }");
        }

        dao = new SoongApiDao(connection);
    }

    @After
    public void tearDown() throws Exception {
        if (connection != null) {
            connection.close();
        }
    }

    private void insertModule(Statement stmt, String name, String json) throws Exception {
        String sql = String.format("INSERT INTO modules (name, metadata) VALUES ('%s', '%s')", name, json);
        stmt.execute(sql);
    }

    @Test
    public void testGetAllModules_returnsAllAggregatedModules() throws Exception {
        // Act
        List<Module> allModules = dao.getAllModules();

        // Assert
        // Total Raw Rows: 5
        // Expected Aggregated Modules: 4
        //   1. libfoo (merged)
        //   2. Videos (mobile)
        //   3. Videos (old)
        //   4. bar
        assertEquals("Should aggregate 5 rows into 4 distinct modules", 4, allModules.size());

        // Verify libfoo aggregation
        Module libfoo = allModules.stream()
                .filter(m -> m.getName().equals("libfoo"))
                .findFirst()
                .orElse(null);
        assertNotNull(libfoo);
        assertEquals(2, libfoo.getInstallFilesCount()); // Merged files

        // Verify Videos separation
        long videosCount = allModules.stream().filter(m -> m.getName().equals("Videos")).count();
        assertEquals(2, videosCount);
    }

    @Test
    public void testGetModule_mergesVariants() throws Exception {
        List<Module> modules = dao.getModule("libfoo");
        assertEquals(1, modules.size());
        assertEquals(2, modules.get(0).getInstallFilesCount());
    }

    @Test
    public void testGetModule_separatesDistinctPaths() throws Exception {
        List<Module> modules = dao.getModule("Videos");
        assertEquals(2, modules.size());
    }

    @Test
    public void testGetModulesByInstallPath() throws Exception {
        List<Module> modules = dao.getModulesByInstallPath("/lib64/libfoo.so");
        assertEquals(1, modules.size());
        assertEquals("libfoo", modules.get(0).getName());
    }
}
