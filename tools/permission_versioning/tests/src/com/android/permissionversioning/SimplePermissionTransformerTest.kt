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

import com.google.common.collect.ImmutableSet
import com.google.common.truth.Truth.assertThat
import java.io.ByteArrayInputStream
import java.nio.charset.StandardCharsets
import java.util.Collections
import javax.xml.parsers.DocumentBuilderFactory
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.junit.runners.JUnit4
import org.w3c.dom.Document
import org.w3c.dom.Element

@RunWith(JUnit4::class)
class SimplePermissionTransformerTest {
    private lateinit var transformer: SimplePermissionTransformer

    @Before
    fun setUp() {
        transformer = SimplePermissionTransformer()
    }

    private fun createDocument(xml: String): Document {
        val factory = DocumentBuilderFactory.newInstance()
        factory.isNamespaceAware = true
        return factory
            .newDocumentBuilder()
            .parse(ByteArrayInputStream(xml.toByteArray(StandardCharsets.UTF_8)))
    }

    @Test
    fun testTransformPermissions_withPermissionRequiringPurposeString() {
        val inputXml =
            "<manifest xmlns:android='http://schemas.android.com/apk/res/android'>" +
                "  <permission android:name='android.permission.TEST'" +
                "              android:protectionLevel='normal'" +
                "              android:requiresPurposeStringTargetSdkVersion='37' />" +
                "</manifest>"
        val inputDoc = createDocument(inputXml)
        val outputDoc = transformer.transformPermissions(inputDoc, Collections.emptySet())

        /*  Expected Artifact XML Representation:
            <permissions>
                <permission name="android.permission.TEST"
                            requiresPurposeStringMinTargetSdkVersion="37" />
            </permissions>
        */
        assertThat(outputDoc).isNotNull()
        assertThat(outputDoc.documentElement.tagName).isEqualTo("permissions")
        assertThat(outputDoc.documentElement.childNodes.length).isEqualTo(1)
        val permissionNodes = outputDoc.getElementsByTagName("permission")
        assertThat(permissionNodes.length).isEqualTo(1)
        val permissionElement = permissionNodes.item(0) as Element
        assertThat(permissionElement.attributes.length).isEqualTo(2)
        assertThat(permissionElement.getAttribute("name")).isEqualTo("android.permission.TEST")
        assertThat(permissionElement.getAttribute("requiresPurposeStringMinTargetSdkVersion"))
            .isEqualTo("37")
        assertThat(permissionElement.childNodes.length).isEqualTo(0)
    }

    @Test
    fun testTransformPermissions_withPermissionRequiringPurposeWithDeprecatedPurpose() {
        val inputXml =
            "<manifest xmlns:android='http://schemas.android.com/apk/res/android'>" +
                "  <permission android:name='android.permission.TEST'" +
                "              android:protectionLevel='normal'" +
                "              android:requiresPurposeTargetSdkVersion='37'>" +
                "    <valid-purpose android:name='purpose1' />" +
                "    <valid-purpose android:name='purpose2' android:maxTargetSdkVersion='38' />" +
                "  </permission>" +
                "</manifest>"
        val inputDoc = createDocument(inputXml)
        val outputDoc = transformer.transformPermissions(inputDoc, Collections.emptySet())

        /*  Expected Artifact XML Representation:
            <permissions>
                <permission name="android.permission.TEST"
                            requiresPurposeMinTargetSdkVersion="37">
                    <valid-purpose name="purpose1" minSdkVersion="37" />
                    <valid-purpose name="purpose2" minSdkVersion="37" maxSdkVersion="38" />
                </permission>
            </permissions>
        */
        assertThat(outputDoc).isNotNull()
        assertThat(outputDoc.documentElement.tagName).isEqualTo("permissions")
        assertThat(outputDoc.documentElement.childNodes.length).isEqualTo(1)
        val permissionNodes = outputDoc.getElementsByTagName("permission")
        assertThat(permissionNodes.length).isEqualTo(1)
        val permissionElement = permissionNodes.item(0) as Element
        assertThat(permissionElement.attributes.length).isEqualTo(2)
        assertThat(permissionElement.getAttribute("name")).isEqualTo("android.permission.TEST")
        assertThat(permissionElement.getAttribute("requiresPurposeMinTargetSdkVersion"))
            .isEqualTo("37")
        assertThat(permissionElement.childNodes.length).isEqualTo(2)
        val purposeNodes = permissionElement.getElementsByTagName("valid-purpose")
        assertThat(purposeNodes.length).isEqualTo(2)
        val purpose1 = purposeNodes.item(0) as Element
        assertThat(purpose1.attributes.length).isEqualTo(2)
        assertThat(purpose1.getAttribute("name")).isEqualTo("purpose1")
        assertThat(purpose1.getAttribute("minSdkVersion")).isEqualTo("37")
        val purpose2 = purposeNodes.item(1) as Element
        assertThat(purpose2.attributes.length).isEqualTo(3)
        assertThat(purpose2.getAttribute("name")).isEqualTo("purpose2")
        assertThat(purpose2.getAttribute("minSdkVersion")).isEqualTo("37")
        assertThat(purpose2.getAttribute("maxSdkVersion")).isEqualTo("38")
    }

    @Test
    fun testTransformPermissions_withPermissionRequiringPurposeAndPurposeStringMigrationFlagDisabled() {
        val inputXml =
            "<manifest xmlns:android='http://schemas.android.com/apk/res/android'>" +
                "  <permission android:name='android.permission.TEST'" +
                "              android:protectionLevel='normal' " +
                "              android:featureFlag='!test.package.flag' />" +
                "  <permission android:name='android.permission.TEST'" +
                "              android:protectionLevel='normal'" +
                "              android:requiresPurposeStringTargetSdkVersion='38'" +
                "              android:requiresPurposeTargetSdkVersion='37'" +
                "              android:featureFlag='test.package.flag'>" +
                "    <valid-purpose android:name='purpose1' />" +
                "  </permission>" +
                "</manifest>"
        val inputDoc = createDocument(inputXml)
        val outputDoc =
            transformer.transformPermissions(
                inputDoc,
                /* enabledFlags= */ ImmutableSet.of("foo.package.flag", "bar.package.flag"),
            )

        // The resultant XML should not contain any permissions i.e. <permissions/>. This is
        // because the older version of android.permission.TEST does not require purpose and that's
        // the version which will be used since test.package.flag is disabled.
        assertThat(outputDoc).isNotNull()
        assertThat(outputDoc.documentElement.tagName).isEqualTo("permissions")
        assertThat(outputDoc.documentElement.childNodes.length).isEqualTo(0)
    }

    @Test
    fun testTransformPermissions_withPermissionRequiringPurposeAndPurposeStringMigrationFlagEnabled() {
        val inputXml =
            "<manifest xmlns:android='http://schemas.android.com/apk/res/android'>" +
                "  <permission android:name='android.permission.TEST'" +
                "              android:protectionLevel='normal' " +
                "              android:featureFlag='!test.package.flag' />" +
                "  <permission android:name='android.permission.TEST'" +
                "              android:protectionLevel='normal'" +
                "              android:requiresPurposeStringTargetSdkVersion='38'" +
                "              android:requiresPurposeTargetSdkVersion='37'" +
                "              android:featureFlag='test.package.flag'>" +
                "    <valid-purpose android:name='purpose1' />" +
                "  </permission>" +
                "</manifest>"
        val inputDoc = createDocument(inputXml)
        val outputDoc =
            transformer.transformPermissions(
                inputDoc,
                /* enabledFlags= */ ImmutableSet.of("foo.package.flag", "test.package.flag"),
            )

        /*  Expected Artifact XML Representation:
            <permissions>
                <permission name="android.permission.TEST"
                            requiresPurposeStringMinTargetSdkVersion="38"
                            requiresPurposeMinTargetSdkVersion="37">
                    <valid-purpose name="purpose1" minSdkVersion="37" />
                </permission>
            </permissions>
        */
        assertThat(outputDoc).isNotNull()
        assertThat(outputDoc.documentElement.tagName).isEqualTo("permissions")
        assertThat(outputDoc.documentElement.childNodes.length).isEqualTo(1)
        val permissionNodes = outputDoc.getElementsByTagName("permission")
        assertThat(permissionNodes.length).isEqualTo(1)
        val permissionElement = permissionNodes.item(0) as Element
        assertThat(permissionElement.attributes.length).isEqualTo(3)
        assertThat(permissionElement.getAttribute("name")).isEqualTo("android.permission.TEST")
        assertThat(permissionElement.getAttribute("requiresPurposeStringMinTargetSdkVersion"))
            .isEqualTo("38")
        assertThat(permissionElement.getAttribute("requiresPurposeMinTargetSdkVersion"))
            .isEqualTo("37")
        assertThat(permissionElement.childNodes.length).isEqualTo(1)
        val purposeNodes = permissionElement.getElementsByTagName("valid-purpose")
        assertThat(purposeNodes.length).isEqualTo(1)
        val purpose1 = purposeNodes.item(0) as Element
        assertThat(purpose1.attributes.length).isEqualTo(2)
        assertThat(purpose1.getAttribute("name")).isEqualTo("purpose1")
        assertThat(purpose1.getAttribute("minSdkVersion")).isEqualTo("37")
    }

    @Test
    fun testTransformPermissions_withMultiplePermissionsRequiringPurpose() {
        val inputXml =
            "<manifest xmlns:android='http://schemas.android.com/apk/res/android'>" +
                "  <permission android:name='android.permission.NO_PURPOSE'" +
                "              android:protectionLevel='normal' />" +
                "  <permission android:name='android.permission.TEST'" +
                "              android:protectionLevel='normal'" +
                "              android:requiresPurposeStringTargetSdkVersion='37' />" +
                "  <permission android:name='android.permission.ANOTHER_TEST'" +
                "              android:protectionLevel='normal'" +
                "              android:requiresPurposeStringTargetSdkVersion='38' />" +
                "</manifest>"
        val inputDoc = createDocument(inputXml)
        val outputDoc = transformer.transformPermissions(inputDoc, Collections.emptySet())

        /*  Expected Artifact XML Representation:
            <permissions>
                <permission name="android.permission.TEST"
                            requiresPurposeStringMinTargetSdkVersion="37" />
                <permission name="android.permission.ANOTHER_TEST"
                            requiresPurposeStringMinTargetSdkVersion="38" />
            </permissions>
        */
        assertThat(outputDoc).isNotNull()
        assertThat(outputDoc.documentElement.tagName).isEqualTo("permissions")
        assertThat(outputDoc.documentElement.childNodes.length).isEqualTo(2)
        val permissionNodes = outputDoc.getElementsByTagName("permission")
        assertThat(permissionNodes.length).isEqualTo(2)
        val permissionElement = permissionNodes.item(0) as Element
        assertThat(permissionElement.attributes.length).isEqualTo(2)
        assertThat(permissionElement.getAttribute("name")).isEqualTo("android.permission.TEST")
        assertThat(permissionElement.getAttribute("requiresPurposeStringMinTargetSdkVersion"))
            .isEqualTo("37")
        assertThat(permissionElement.childNodes.length).isEqualTo(0)
        val permissionElement2 = permissionNodes.item(1) as Element
        assertThat(permissionElement2.attributes.length).isEqualTo(2)
        assertThat(permissionElement2.getAttribute("name"))
            .isEqualTo("android.permission.ANOTHER_TEST")
        assertThat(permissionElement2.getAttribute("requiresPurposeStringMinTargetSdkVersion"))
            .isEqualTo("38")
        assertThat(permissionElement2.childNodes.length).isEqualTo(0)
    }
}
