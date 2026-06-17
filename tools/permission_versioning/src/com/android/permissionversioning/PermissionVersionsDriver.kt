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
@file:JvmName("PermissionVersionsDriver")

package com.android.permissionversioning

import java.nio.file.Paths
import javax.xml.parsers.DocumentBuilderFactory
import javax.xml.transform.OutputKeys
import javax.xml.transform.TransformerFactory
import javax.xml.transform.dom.DOMSource
import javax.xml.transform.stream.StreamResult
import kotlin.system.exitProcess

/**
 * Main class that orchestrates the generation of XML file containing SDK versioned info for
 * permissions requiring purpose based on the declarations in the platform manifest XML.
 */
fun main(args: Array<String>) {
    if (args.size != 3) {
        System.err.println(
            "Usage: PermissionVersionsDriver <manifest_path> <aconfig_flags_pb_path> <output_path>"
        )
        exitProcess(1)
    }

    val inputXmlPath = Paths.get(args[0])
    val flagsInputPath = Paths.get(args[1])
    val outputFilePath = Paths.get(args[2])

    // 1. Get enabled aconfig flags for the build configuration
    val enabledFlags = FlagParser().getEnabledFlags(flagsInputPath)

    // 2. Parse the input manifest XML
    val factory = DocumentBuilderFactory.newInstance().apply { isNamespaceAware = true }
    val builder = factory.newDocumentBuilder()
    val inputDoc = builder.parse(inputXmlPath.toFile())

    // 3. Transform manifest XML to generate a versioned doc with permissions and purposes.
    val outputDoc = SimplePermissionTransformer().transformPermissions(inputDoc, enabledFlags)

    // 4. Write the generated XML document
    val transformerFactory = TransformerFactory.newInstance()
    val transformerWriter =
        transformerFactory.newTransformer().apply {
            setOutputProperty(OutputKeys.INDENT, "yes")
            setOutputProperty("{http://xml.apache.org/xslt}indent-amount", "2")
        }
    transformerWriter.transform(DOMSource(outputDoc), StreamResult(outputFilePath.toFile()))
}
