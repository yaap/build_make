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

import javax.xml.parsers.DocumentBuilderFactory
import org.w3c.dom.Document
import org.w3c.dom.Element
import org.w3c.dom.Node

/**
 * Contains logic to transform platform-defined permission elements requiring purpose into a new XML
 * document with appropriate SDK versioning. The source of truth versioning is directly inferred
 * from the `requires*TargetSdkVersion` attribute on the {@code <permission>} element.
 */
// TODO(b/454111429): Enhance versioning logic to handle updating preexisting purpose guarded
//  permissions with new purposes, permission deletions, and converting a permission to no longer
//  require purpose in future SDKs.
class SimplePermissionTransformer {
    /**
     * Transforms the platform manifest XML into a custom XML artifact containing all permissions
     * and purposes with their associated min / max SDK versions.
     *
     * @param inputDoc parsed platform manifest XML document
     * @param enabledFlags set of framework flags (package.name) that are enabled.
     * @return document representing the XML artifact containing versioned permissions and purposes.
     */
    fun transformPermissions(inputDoc: Document, enabledFlags: Set<String>): Document {
        val outputDoc =
            DocumentBuilderFactory.newInstance()
                .apply { isNamespaceAware = true }
                .newDocumentBuilder()
                .newDocument()
        val rootElement = outputDoc.createElement(ARTIFACT_TAG_ROOT)
        outputDoc.appendChild(rootElement)
        processPermissionNodes(inputDoc, enabledFlags, outputDoc, rootElement)
        return outputDoc
    }

    private fun processPermissionNodes(
        inputDoc: Document,
        enabledFlags: Set<String>,
        outputDoc: Document,
        outputRoot: Element,
    ) {
        val permissionNodes = inputDoc.getElementsByTagName(TAG_PERMISSION)
        for (i in 0..<permissionNodes.length) {
            val node = permissionNodes.item(i)
            if (node.nodeType == Node.ELEMENT_NODE) {
                val permissionElement = node as Element
                if (shouldIncludePermission(permissionElement, enabledFlags)) {
                    transformAndAppendPermission(permissionElement, outputDoc, outputRoot)
                }
            }
        }
    }

    private fun shouldIncludePermission(
        permissionElement: Element,
        enabledFlags: Set<String>,
    ): Boolean {
        val requiresPurpose =
            permissionElement
                .getAttributeNS(ANDROID_NS, MANIFEST_ATTR_REQUIRES_PURPOSE)
                .isNotEmpty()
        val requiresPurposeString =
            permissionElement
                .getAttributeNS(ANDROID_NS, MANIFEST_ATTR_REQUIRES_PURPOSE_STRING)
                .isNotEmpty()
        if (!requiresPurpose && !requiresPurposeString) {
            return false
        }
        val featureFlag = permissionElement.getAttributeNS(ANDROID_NS, MANIFEST_ATTR_FEATURE_FLAG)
        if (featureFlag.isEmpty()) {
            return true
        }

        // If the flag is negated, the flag must be disabled for the element to be "active"
        return if (featureFlag.startsWith("!")) !enabledFlags.contains(featureFlag.substring(1))
        else enabledFlags.contains(featureFlag)
    }

    private fun transformAndAppendPermission(
        inputPermissionElement: Element,
        outputDoc: Document,
        outputRoot: Element,
    ) {
        // Create permission element and set permission name
        val permissionName = inputPermissionElement.getAttributeNS(ANDROID_NS, ATTR_NAME)
        val newPermissionElement = outputDoc.createElement(TAG_PERMISSION)
        newPermissionElement.setAttribute(ATTR_NAME, permissionName)

        // Set purpose string requirement min SDK version attribute
        val requiresPurposeStringTargetSdkVersion =
            inputPermissionElement.getAttributeNS(ANDROID_NS, MANIFEST_ATTR_REQUIRES_PURPOSE_STRING)
        if (!requiresPurposeStringTargetSdkVersion.isEmpty()) {
            newPermissionElement.setAttribute(
                ARTIFACT_ATTR_PURPOSE_STRING_MIN,
                requiresPurposeStringTargetSdkVersion,
            )
        }

        // Set purpose requirement min SDK version attribute
        val requiresPurposeTargetSdkVersion =
            inputPermissionElement.getAttributeNS(ANDROID_NS, MANIFEST_ATTR_REQUIRES_PURPOSE)
        if (requiresPurposeTargetSdkVersion.isEmpty()) {
            outputRoot.appendChild(newPermissionElement)
            return
        }
        newPermissionElement.setAttribute(
            ARTIFACT_ATTR_REQUIRES_PURPOSE_MIN,
            requiresPurposeTargetSdkVersion,
        )

        // Version the declared valid purposes
        val purposeNodes = inputPermissionElement.getElementsByTagName(TAG_VALID_PURPOSE)
        for (i in 0..<purposeNodes.length) {
            val purposeNode = purposeNodes.item(i)
            if (purposeNode.nodeType == Node.ELEMENT_NODE) {
                copyPurposeElement(
                    purposeNode as Element,
                    outputDoc,
                    newPermissionElement,
                    requiresPurposeTargetSdkVersion,
                )
            }
        }
        outputRoot.appendChild(newPermissionElement)
    }

    private fun copyPurposeElement(
        purposeElement: Element,
        outputDoc: Document,
        newPermissionElement: Element,
        parentMinSdkVersion: String,
    ) {
        // Create valid-purpose child element and set name
        val newPurposeElement = outputDoc.createElement(TAG_VALID_PURPOSE)
        val purposeName = purposeElement.getAttributeNS(ANDROID_NS, ATTR_NAME)
        newPurposeElement.setAttribute(ATTR_NAME, purposeName)

        // Set min SDK version attribute for purpose. For now, it will be the same as
        // the requiresPurposeTargetSdkVersion attribute from the permission element.
        newPurposeElement.setAttribute(ARTIFACT_ATTR_PURPOSE_MIN, parentMinSdkVersion)

        // If defined, set max SDK version for purpose to mark as deprecated.
        val maxTargetSdkVersion =
            purposeElement.getAttributeNS(ANDROID_NS, MANIFEST_ATTR_PURPOSE_MAX)
        if (!maxTargetSdkVersion.isEmpty()) {
            newPurposeElement.setAttribute(ARTIFACT_ATTR_PURPOSE_MAX, maxTargetSdkVersion)
        }

        // TODO(b/454115740): Resolve any flag associated with the purpose tag before adding when
        //  versioning new purposes to a preexisting purpose guarded permission is supported.
        newPermissionElement.appendChild(newPurposeElement)
    }

    companion object {
        private const val ANDROID_NS = "http://schemas.android.com/apk/res/android"

        // Tags & attributes for manifest XML
        private const val MANIFEST_ATTR_FEATURE_FLAG = "featureFlag"
        private const val MANIFEST_ATTR_PURPOSE_MAX = "maxTargetSdkVersion"
        private const val MANIFEST_ATTR_REQUIRES_PURPOSE = "requiresPurposeTargetSdkVersion"
        private const val MANIFEST_ATTR_REQUIRES_PURPOSE_STRING =
            "requiresPurposeStringTargetSdkVersion"

        // Tags & attributes for artifact XML
        private const val ARTIFACT_TAG_ROOT = "permissions"
        private const val ARTIFACT_ATTR_REQUIRES_PURPOSE_MIN = "requiresPurposeMinTargetSdkVersion"
        private const val ARTIFACT_ATTR_PURPOSE_STRING_MIN =
            "requiresPurposeStringMinTargetSdkVersion"
        private const val ARTIFACT_ATTR_PURPOSE_MIN = "minSdkVersion"
        private const val ARTIFACT_ATTR_PURPOSE_MAX = "maxSdkVersion"

        // Common tags & attributes for manifest & artifact XML
        private const val TAG_VALID_PURPOSE = "valid-purpose"
        private const val TAG_PERMISSION = "permission"
        private const val ATTR_NAME = "name"
    }
}
