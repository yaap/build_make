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
import com.android.soong.api.proto.GetAllModulesRequest;
import com.android.soong.api.proto.GetModuleByInstallPathRequest;
import com.android.soong.api.proto.GetModuleRequest;
import com.android.soong.api.proto.SoongApiServiceGrpc;
import com.android.soong.api.proto.Module;

import io.grpc.Status;
import io.grpc.stub.StreamObserver;

import java.sql.SQLException;
import java.util.List;

/**
 * Implementation of the gRPC SoongApiService.
 */
public class SoongApiServiceImpl extends SoongApiServiceGrpc.SoongApiServiceImplBase {

    private final SoongApiDao dao;

    public SoongApiServiceImpl(SoongApiDao dao) {
        this.dao = dao;
    }

    /**
     * Streams <b>all</b> aggregated modules found in the database.
     */
    @Override
    public void getAllModules(GetAllModulesRequest request, StreamObserver<Module> responseObserver) {
        try {
            List<Module> modules = dao.getAllModules();
            for (Module module : modules) {
                responseObserver.onNext(module);
            }
            responseObserver.onCompleted();
        } catch (SQLException e) {
            responseObserver.onError(Status.INTERNAL
                .withDescription("Database error during getAllModules: " + e.getMessage())
                .asRuntimeException());
        }
    }

    /**
     * Streams modules matching the given name.
     * <p>
     * Returns a stream because a single name (e.g., "Videos") might map to multiple
     * distinct module definitions (e.g., different source paths).
     */
    @Override
    public void getModule(GetModuleRequest request, StreamObserver<Module> responseObserver) {
        try {
            List<Module> modules = dao.getModule(request.getName());
            for (Module module : modules) {
                responseObserver.onNext(module);
            }
            responseObserver.onCompleted();
        } catch (SQLException e) {
            responseObserver.onError(Status.INTERNAL
                .withDescription("Database error during getModule: " + e.getMessage())
                .asRuntimeException());
        }
    }

    /**
     * Streams modules that install the specified file.
     * <p>
     * Returns a stream because multiple modules may claim to install the same file.
     */
    @Override
    public void getModuleByInstallPath(GetModuleByInstallPathRequest request, StreamObserver<Module> responseObserver) {
        try {
            List<Module> modules = dao.getModulesByInstallPath(request.getInstallPath());
            for (Module module : modules) {
                responseObserver.onNext(module);
            }
            responseObserver.onCompleted();
        } catch (SQLException e) {
            responseObserver.onError(Status.INTERNAL
                .withDescription("Database error during getModuleByInstallPath: " + e.getMessage())
                .asRuntimeException());
        }
    }
}
