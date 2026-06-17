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

package com.android.aconfig.diff.test;

import static com.google.common.truth.Truth.assertWithMessage;

import com.android.tradefed.testtype.DeviceJUnit4ClassRunner;
import com.android.tradefed.testtype.junit4.BaseHostJUnit4Test;
import com.android.tradefed.util.CommandResult;
import com.android.tradefed.util.CommandStatus;
import com.android.tradefed.util.FileUtil;
import com.android.tradefed.util.RunUtil;

import org.junit.Test;
import org.junit.runner.RunWith;

import java.io.File;

/** Host-side test to verify aconfig flag optimization impacts on DEX output. */
@RunWith(DeviceJUnit4ClassRunner.class)
public class AconfigDiffHostTest extends BaseHostJUnit4Test {

    private static final String DEXDUMP = "dexdump";

    // -------------------------------------------------------------------------
    // Whitespace-only changes: Expect No Functional Diff
    // -------------------------------------------------------------------------

    @Test
    public void testWhitespace_NoOpt() throws Exception {
        verifyZeroDiff("whitespace", "no_opt");
    }

    @Test
    public void testWhitespace_ShrinkOpt() throws Exception {
        verifyZeroDiff("whitespace", "shrink_opt");
    }

    @Test
    public void testWhitespace_FullOpt() throws Exception {
        verifyZeroDiff("whitespace", "full_opt");
    }

    // -------------------------------------------------------------------------
    // RO-flag Branch changes: Expect functional diff only when NOT optimized
    // -------------------------------------------------------------------------

    @Test
    public void testBranch_RO_NoOpt() throws Exception {
        verifyDiff("branch_ro", "no_opt");
    }

    @Test
    public void testBranch_RO_ShrinkOpt() throws Exception {
        verifyZeroDiff("branch_ro", "shrink_opt");
    }

    @Test
    public void testBranch_RO_FullOpt() throws Exception {
        verifyZeroDiff("branch_ro", "full_opt");
    }

    // -------------------------------------------------------------------------
    // RW-flag Branch changes: Expect functional diffs (RW check always remains)
    // -------------------------------------------------------------------------

    @Test
    public void testBranch_RW_NoOpt() throws Exception {
        verifyDiff("branch_rw", "no_opt");
    }

    @Test
    public void testBranch_RW_ShrinkOpt() throws Exception {
        verifyDiff("branch_rw", "shrink_opt");
    }

    @Test
    public void testBranch_RW_FullOpt() throws Exception {
        verifyDiff("branch_rw", "full_opt");
    }

    // -------------------------------------------------------------------------
    // RO-flag Class addition: Expect functional diff only when NOT optimized
    // -------------------------------------------------------------------------

    @Test
    public void testClass_RO_NoOpt() throws Exception {
        verifyDiff("class_ro", "no_opt");
    }

    @Test
    public void testClass_RO_ShrinkOpt() throws Exception {
        verifyZeroDiff("class_ro", "shrink_opt");
    }

    @Test
    public void testClass_RO_FullOpt() throws Exception {
        verifyZeroDiff("class_ro", "full_opt");
    }

    /** Asserts that the two targets have functionally identical DEX output. */
    private void verifyZeroDiff(String testCase, String optLevel) throws Exception {
        compare(testCase, optLevel, /* expectMatch= */ true);
    }

    /** Asserts that the two targets have functionally different DEX output. */
    private void verifyDiff(String testCase, String optLevel) throws Exception {
        compare(testCase, optLevel, /* expectMatch= */ false);
    }

    /** Internal comparison engine. */
    private void compare(String testCase, String optLevel, boolean expectMatch) throws Exception {
        String baseName = String.format("aconfig_diff_test_base_%s", optLevel);
        String testName = String.format("aconfig_diff_test_%s_%s", testCase, optLevel);

        File baseApk = getFile(baseName + ".apk");
        File testApk = getFile(testName + ".apk");

        // 1. Bitwise Shortcut
        boolean bitwiseMatch = FileUtil.compareFileContents(baseApk, testApk);
        if (bitwiseMatch) {
            assertWithMessage(
                            "Found bitwise match for %s vs %s, but a functional diff was expected.",
                            baseName, testName)
                    .that(expectMatch)
                    .isTrue();
            return;
        }

        // 2. Functional Comparison (Dexdump sans debug info)
        String baseNorm = runDexdump(baseApk);
        String testNorm = runDexdump(testApk);

        if (expectMatch) {
            assertWithMessage("Functional diff found between %s and %s", baseName, testName)
                    .that(testNorm)
                    .isEqualTo(baseNorm);
        } else {
            assertWithMessage(
                            "Expected a functional diff between %s and %s, but they are identical.",
                            baseName, testName)
                    .that(testNorm)
                    .isNotEqualTo(baseNorm);
        }
    }

    private String runDexdump(File apk) throws Exception {
        File dexdump = getDexdump();
        // -d: disassembles code sections (we want to diff actual code but in a readable way!)
        // -n: disable debug info output (e.g., file paths, line numbers)
        String[] cmd = {dexdump.getAbsolutePath(), "-d", "-n", apk.getAbsolutePath()};
        CommandResult result = RunUtil.getDefault().runTimedCmd(60000, cmd);

        if (result.getStatus() != CommandStatus.SUCCESS) {
            throw new RuntimeException("dexdump failed: " + result.getStderr());
        }
        return result.getStdout();
    }

    private File getFile(String name) {
        try {
            return getTestInformation().getDependencyFile(name, true);
        } catch (Exception e) {
            throw new RuntimeException("Failed to find test file: " + name, e);
        }
    }

    private File getDexdump() {
        try {
            File dexdump = getTestInformation().getDependencyFile(DEXDUMP, false);
            if (dexdump != null && dexdump.exists()) {
                dexdump.setExecutable(true);
                return dexdump;
            }
        } catch (Exception ignored) {
        }
        return new File(DEXDUMP);
    }
}
