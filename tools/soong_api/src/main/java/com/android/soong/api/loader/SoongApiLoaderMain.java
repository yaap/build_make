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

import java.io.File;

/**
 * CLI entry point for the Soong API Database Loader.
 */
public class SoongApiLoaderMain {
    public static void main(String[] args) {
        String inputPath = null;
        String dbPath = null;

        for (int i = 0; i < args.length; i++) {
            if ("-i".equals(args[i]) && i + 1 < args.length) {
                inputPath = args[i + 1];
                i++;
            } else if ("-o".equals(args[i]) && i + 1 < args.length) {
                dbPath = args[i + 1];
                i++;
            }
        }

        if (inputPath == null || dbPath == null) {
            System.err.println("Usage: soong_api_db_loader -i <input.zip or input.json> -o <output.db>");
            System.exit(1);
        }

        try {
            SoongApiLoader loader = new SoongApiLoader();
            loader.load(new File(inputPath), new File(dbPath));
            System.out.println("Successfully generated Soong API database: " + dbPath);
        } catch (Exception e) {
            e.printStackTrace();
            System.exit(1);
        }
    }
}
