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

import com.android.soong.api.proto.Module;
import com.google.protobuf.util.JsonFormat;
import java.util.Iterator;

public class SoongApiClientExample {

    public static void main(String[] args) {
        // Configures a JSON printer that preserves proto field names (e.g., "install_files")
        // instead of converting to camelCase, matching the raw DB/Evans output style.
        JsonFormat.Printer jsonPrinter = JsonFormat.printer()
                .preservingProtoFieldNames()
                .omittingInsignificantWhitespace();

        System.out.println("Initializing Soong API Client...");

        // Try-with-resources ensures the client (and the server process) is closed automatically.
        try (SoongApiClient client = new SoongApiClient()) {

            // ==========================================
            // Example 1: Query by Module Name
            // ==========================================
            String queryName = "Videos"; // Default query
            if (args.length > 0) {
                queryName = args[0];
            }

            System.out.println("\n[1] Querying GetModule (Name: " + queryName + ")");
            System.out.println("--------------------------------------------------");

            // Use the convenience method directly
            Iterator<Module> nameResponses = client.getModule(queryName);

            while (nameResponses.hasNext()) {
                printModule(jsonPrinter, nameResponses.next());
            }

            // ==========================================
            // Example 2: Query by Install Path
            // ==========================================
            String installPath = "out/target/product/vsoc_x86_64/system_other/product/app/Videos/oat/x86_64/Videos.odex";

            System.out.println("\n[2] Querying GetModuleByInstallPath (Path: " + installPath + ")");
            System.out.println("--------------------------------------------------");

            // Use the convenience method directly
            Iterator<Module> pathResponses = client.getModuleByInstallPath(installPath);

            while (pathResponses.hasNext()) {
                printModule(jsonPrinter, pathResponses.next());
            }

            System.out.println("--------------------------------------------------");
            System.out.println("Done.");

        } catch (Exception e) {
            System.err.println("Fatal Error: " + e.getMessage());
            e.printStackTrace();
            System.exit(1);
        }
    }

    private static void printModule(JsonFormat.Printer printer, Module module) {
        try {
            System.out.println(printer.print(module));
        } catch (Exception e) {
            System.err.println("Error printing module: " + e.getMessage());
        }
    }
}
