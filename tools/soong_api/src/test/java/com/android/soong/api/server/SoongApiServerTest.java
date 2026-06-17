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

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;
import static org.junit.Assert.assertFalse;

import com.android.soong.api.db.SoongApiDao;
import com.android.soong.api.proto.GetAllModulesRequest;
import com.android.soong.api.proto.GetModuleByInstallPathRequest;
import com.android.soong.api.proto.GetModuleRequest;
import com.android.soong.api.proto.SoongApiServiceGrpc;
import com.android.soong.api.proto.Module;

import io.grpc.ManagedChannel;
import io.grpc.inprocess.InProcessChannelBuilder;
import io.grpc.inprocess.InProcessServerBuilder;
import io.grpc.testing.GrpcCleanupRule;

import org.junit.After;
import org.junit.Before;
import org.junit.Rule;
import org.junit.Test;
import org.junit.runner.RunWith;
import org.junit.runners.JUnit4;

import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.Statement;
import java.util.Iterator;
import java.util.ArrayList;
import java.util.List;

/**
 * Tests for {@link SoongApiServiceImpl} using an in-process gRPC server.
 */
@RunWith(JUnit4.class)
public class SoongApiServerTest {

    @Rule
    public final GrpcCleanupRule grpcCleanup = new GrpcCleanupRule();

    private Connection connection;
    private SoongApiServiceGrpc.SoongApiServiceBlockingStub blockingStub;

    @Before
    public void setUp() throws Exception {
        connection = DriverManager.getConnection("jdbc:sqlite::memory:");

        try (Statement stmt = connection.createStatement()) {
            stmt.execute("CREATE TABLE modules (name TEXT, metadata TEXT)");
            stmt.execute("CREATE INDEX idx_name ON modules(name)");

            String jsonFoo = "{ \"name\": \"libfoo\", \"type\": \"cc_library\", \"path\": \"src/foo\", " +
                             "  \"install_files\": [\"/system/lib64/libfoo.so\"] }";
            stmt.execute(String.format("INSERT INTO modules VALUES ('libfoo', '%s')", jsonFoo));

            String jsonBar = "{ \"name\": \"app_bar\", \"type\": \"android_app\", \"path\": \"src/bar\", " +
                             "  \"install_files\": [\"/system/app/Bar.apk\"] }";
            stmt.execute(String.format("INSERT INTO modules VALUES ('app_bar', '%s')", jsonBar));
        }

        SoongApiDao dao = new SoongApiDao(connection);
        SoongApiServiceImpl serviceImpl = new SoongApiServiceImpl(dao);

        String serverName = InProcessServerBuilder.generateName();

        grpcCleanup.register(InProcessServerBuilder
                .forName(serverName)
                .directExecutor()
                .addService(serviceImpl)
                .build()
                .start());

        ManagedChannel channel = grpcCleanup.register(InProcessChannelBuilder
                .forName(serverName)
                .directExecutor()
                .build());

        blockingStub = SoongApiServiceGrpc.newBlockingStub(channel);
    }

    @After
    public void tearDown() throws Exception {
        if (connection != null) {
            connection.close();
        }
    }

    @Test
    public void testGetAllModules_returnsAllData() {
        GetAllModulesRequest request = GetAllModulesRequest.newBuilder().build();
        Iterator<Module> iterator = blockingStub.getAllModules(request);

        List<Module> modules = new ArrayList<>();
        iterator.forEachRemaining(modules::add);

        assertEquals(2, modules.size());
        assertTrue(modules.stream().anyMatch(m -> m.getName().equals("libfoo")));
    }

    @Test
    public void testGetModule_returnsSpecificData() {
        GetModuleRequest request = GetModuleRequest.newBuilder().setName("libfoo").build();
        Iterator<Module> iterator = blockingStub.getModule(request);

        assertTrue(iterator.hasNext());
        Module module = iterator.next();
        assertEquals("libfoo", module.getName());
        assertEquals("cc_library", module.getType());
    }

    @Test
    public void testGetModuleByInstallPath_returnsCorrectModule() {
        GetModuleByInstallPathRequest request = GetModuleByInstallPathRequest.newBuilder()
                .setInstallPath("/system/app/Bar.apk")
                .build();

        Iterator<Module> iterator = blockingStub.getModuleByInstallPath(request);
        assertTrue(iterator.hasNext());
        assertEquals("app_bar", iterator.next().getName());
    }
}
