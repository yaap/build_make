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

import com.android.soong.api.proto.Module;
import com.google.protobuf.util.JsonFormat;

import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;

/**
 * Data Access Object for Soong API metadata stored in SQLite.
 */
public class SoongApiDao {
    private final Connection connection;

    /**
     * @param connection The JDBC connection to the SQLite database.
     */
    public SoongApiDao(Connection connection) {
        this.connection = connection;
    }

    /**
     * Retrieves all aggregated modules from the database.
     */
    public List<Module> getAllModules() throws SQLException {
        String sql = "SELECT metadata FROM modules";
        return queryAndAggregate(sql, null);
    }

    /**
     * Retrieves modules by name, merging variants from the same source path.
     */
    public List<Module> getModule(String name) throws SQLException {
        String sql = "SELECT metadata FROM modules WHERE name = ?";
        return queryAndAggregate(sql, name);
    }

    /**
     * Retrieves modules that install a specific file.
     */
    public List<Module> getModulesByInstallPath(String installPath) throws SQLException {
        String sql = "SELECT metadata FROM modules WHERE EXISTS (" +
                     "  SELECT 1 FROM json_each(modules.metadata, '$.install_files') " +
                     "  WHERE value = ?" +
                     ")";
        return queryAndAggregate(sql, installPath);
    }

    /**
     * Executes the query and aggregates the results based on (Name + Path).
     *
     * <p>Uses JsonFormat for programmatic conversion from JSON string to Protobuf messages.
     */
    private List<Module> queryAndAggregate(String sql, String param) throws SQLException {
        Map<String, Module.Builder> builders = new HashMap<>();
        Map<String, Set<String>> fileSets = new HashMap<>();
        JsonFormat.Parser parser = JsonFormat.parser().ignoringUnknownFields();

        try (PreparedStatement pstmt = connection.prepareStatement(sql)) {
            if (param != null) {
                pstmt.setString(1, param);
            }

            try (ResultSet rs = pstmt.executeQuery()) {
                while (rs.next()) {
                    String jsonStr = rs.getString("metadata");

                    // Programmatic conversion: JSON -> Proto Builder
                    Module.Builder tempBuilder = Module.newBuilder();
                    try {
                        parser.merge(jsonStr, tempBuilder);
                    } catch (Exception e) {
                        throw new SQLException("Error converting JSON to Protobuf", e);
                    }

                    String name = tempBuilder.getName();
                    String path = tempBuilder.getPath();

                    // Composite Key for Aggregation (Same name, same source path = Same module)
                    String aggregationKey = name + "|" + path;

                    builders.putIfAbsent(aggregationKey, Module.newBuilder());
                    fileSets.putIfAbsent(aggregationKey, new HashSet<>());

                    Module.Builder mainBuilder = builders.get(aggregationKey);

                    // Set fields if they are not yet populated or might have changed in variants
                    mainBuilder.setName(name);
                    mainBuilder.setPath(path);
                    if (!tempBuilder.getType().isEmpty()) {
                        mainBuilder.setType(tempBuilder.getType());
                    }

                    // Aggregate install_files into the set
                    fileSets.get(aggregationKey).addAll(tempBuilder.getInstallFilesList());
                }
            }
        }

        List<Module> results = new ArrayList<>();
        for (String key : builders.keySet()) {
            Module.Builder builder = builders.get(key);
            builder.addAllInstallFiles(fileSets.get(key));
            results.add(builder.build());
        }
        return results;
    }
}
