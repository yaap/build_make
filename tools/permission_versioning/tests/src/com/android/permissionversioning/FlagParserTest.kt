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
import android.aconfig.Aconfig.parsed_flag
import android.aconfig.Aconfig.parsed_flags
import com.google.common.truth.Truth.assertThat
import java.io.File
import java.io.FileOutputStream
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.junit.runners.JUnit4

@RunWith(JUnit4::class)
class FlagParserTest {
    @JvmField @Rule var tempFolder = TemporaryFolder()

    private lateinit var flagParser: FlagParser

    @Before
    fun setUp() {
        flagParser = FlagParser()
    }

    @Test
    fun testGetEnabledFlags() {
        val enabledFlag1 =
            parsed_flag
                .newBuilder()
                .setPackage("com.example")
                .setName("enabled_flag_1")
                .setState(flag_state.ENABLED)
                .build()
        val enabledFlag2 =
            parsed_flag
                .newBuilder()
                .setPackage("com.example")
                .setName("enabled_flag_2")
                .setState(flag_state.ENABLED)
                .build()
        val disabledFlag =
            parsed_flag
                .newBuilder()
                .setPackage("com.example")
                .setName("disabled_flag")
                .setState(flag_state.DISABLED)
                .build()
        val flags =
            parsed_flags
                .newBuilder()
                .addParsedFlag(enabledFlag1)
                .addParsedFlag(enabledFlag2)
                .addParsedFlag(disabledFlag)
                .build()
        val tempFile: File = tempFolder.newFile("flags.pb")
        flags.writeTo(FileOutputStream(tempFile))
        val flagsFilePath = tempFile.toPath()
        val enabledFlags = flagParser.getEnabledFlags(flagsFilePath)
        assertThat(enabledFlags).hasSize(2)
        assertThat(enabledFlags).contains("com.example.enabled_flag_1")
        assertThat(enabledFlags).contains("com.example.enabled_flag_2")
    }

    @Test
    fun testGetEnabledFlags_noEnabledFlags() {
        val disabledFlag1 =
            parsed_flag
                .newBuilder()
                .setPackage("com.example")
                .setName("disabled_flag_1")
                .setState(flag_state.DISABLED)
                .build()
        val disabledFlag2 =
            parsed_flag
                .newBuilder()
                .setPackage("com.example")
                .setName("disabled_flag_2")
                .setState(flag_state.DISABLED)
                .build()
        val flags =
            parsed_flags
                .newBuilder()
                .addParsedFlag(disabledFlag1)
                .addParsedFlag(disabledFlag2)
                .build()
        val tempFile: File = tempFolder.newFile("flags.pb")
        flags.writeTo(FileOutputStream(tempFile))
        val flagsFilePath = tempFile.toPath()
        val enabledFlags = flagParser.getEnabledFlags(flagsFilePath)
        assertThat(enabledFlags).isEmpty()
    }

    @Test
    fun testGetEnabledFlags_emptyFile() {
        val flags = parsed_flags.newBuilder().build()
        val tempFile: File = tempFolder.newFile("flags.pb")
        flags.writeTo(FileOutputStream(tempFile))
        val flagsFilePath = tempFile.toPath()
        val enabledFlags = flagParser.getEnabledFlags(flagsFilePath)
        assertThat(enabledFlags).isEmpty()
    }
}
