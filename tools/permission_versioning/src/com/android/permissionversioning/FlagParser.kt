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

package com.android.permissionversioning

import android.aconfig.Aconfig.flag_state
import android.aconfig.Aconfig.parsed_flags
import java.nio.file.Files
import java.nio.file.Path

/** Helper class to parse a protobuf file containing feature flags. */
class FlagParser {
    /**
     * Reads the flags file and returns a set of fully qualified names of enabled flags.
     *
     * @param flagsFilePath Path to the aconfig flags proto file.
     * @return Set of enabled flag names (e.g., "package.name").
     */
    fun getEnabledFlags(flagsFilePath: Path): Set<String> {
        val enabledFlags = mutableSetOf<String>()
        val parsedFlags =
            Files.newInputStream(flagsFilePath).use { inputStream ->
                parsed_flags.parseFrom(inputStream)
            }
        for (flag in parsedFlags.parsedFlagList) {
            if (flag.state == flag_state.ENABLED) {
                val fullFlagName = "${flag.getPackage()}.${flag.name}"
                enabledFlags.add(fullFlagName)
            }
        }
        return enabledFlags
    }
}
