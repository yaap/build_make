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

package android.util;

public final class StatsLog {
    public static final byte ANNOTATION_ID_IS_UID = 1;
    public static final byte ANNOTATION_ID_TRUNCATE_TIMESTAMP = 2;
    public static final byte ANNOTATION_ID_PRIMARY_FIELD = 3;
    public static final byte ANNOTATION_ID_EXCLUSIVE_STATE = 4;
    public static final byte ANNOTATION_ID_PRIMARY_FIELD_FIRST_UID = 5;
    public static final byte ANNOTATION_ID_DEFAULT_STATE = 6;
    public static final byte ANNOTATION_ID_TRIGGER_STATE_RESET = 7;
    public static final byte ANNOTATION_ID_STATE_NESTED = 8;
    public static final byte ANNOTATION_ID_RESTRICTION_CATEGORY = 9;
    public static final byte ANNOTATION_ID_FIELD_RESTRICTION_PERIPHERAL_DEVICE_INFO = 10;
    public static final byte ANNOTATION_ID_FIELD_RESTRICTION_APP_USAGE = 11;
    public static final byte ANNOTATION_ID_FIELD_RESTRICTION_APP_ACTIVITY = 12;
    public static final byte ANNOTATION_ID_FIELD_RESTRICTION_HEALTH_CONNECT = 13;
    public static final byte ANNOTATION_ID_FIELD_RESTRICTION_ACCESSIBILITY = 14;
    public static final byte ANNOTATION_ID_FIELD_RESTRICTION_SYSTEM_SEARCH = 15;
    public static final byte ANNOTATION_ID_FIELD_RESTRICTION_USER_ENGAGEMENT = 16;
    public static final byte ANNOTATION_ID_FIELD_RESTRICTION_AMBIENT_SENSING = 17;
    public static final byte ANNOTATION_ID_FIELD_RESTRICTION_DEMOGRAPHIC_CLASSIFICATION = 18;

    public static final int RESTRICTION_CATEGORY_DIAGNOSTIC = 1;
    public static final int RESTRICTION_CATEGORY_SYSTEM_INTELLIGENCE = 2;
    public static final int RESTRICTION_CATEGORY_AUTHENTICATION = 3;
    public static final int RESTRICTION_CATEGORY_FRAUD_AND_ABUSE = 4;

    private StatsLog() {
        throw new RuntimeException("Stub!");
    }

    /**
     * Logs a start event.
     *
     * @param label developer-chosen label.
     * @return True if the log request was sent to statsd.
     */
    public static boolean logStart(int label) {
        throw new RuntimeException("Stub!");
    }

    /**
     * Logs a stop event.
     *
     * @param label developer-chosen label.
     * @return True if the log request was sent to statsd.
     */
    public static boolean logStop(int label) {
        throw new RuntimeException("Stub!");
    }

    /**
     * Logs an event that does not represent a start or stop boundary.
     *
     * @param label developer-chosen label.
     * @return True if the log request was sent to statsd.
     */
    public static boolean logEvent(int label) {
        throw new RuntimeException("Stub!");
    }

    /**
     * Logs an event for binary push for module updates.
     *
     * @param trainName        name of install train.
     * @param trainVersionCode version code of the train.
     * @param options          optional flags about this install.
     *                         The last 3 bits indicate options:
     *                         0x01: FLAG_REQUIRE_STAGING
     *                         0x02: FLAG_ROLLBACK_ENABLED
     *                         0x04: FLAG_REQUIRE_LOW_LATENCY_MONITOR
     * @param state            current install state. Defined as State enums in
     *                         BinaryPushStateChanged atom in
     *                         frameworks/proto_logging/stats/atoms.proto
     * @param experimentIds    experiment ids.
     * @return True if the log request was sent to statsd.
     */
    public static boolean logBinaryPushStateChanged(String trainName, long trainVersionCode,
        int options, int state, long[] experimentIds) {
        throw new RuntimeException("Stub!");
    }

    /**
     * Write an event to stats log using the raw format.
     *
     * @param buffer The encoded buffer of data to write.
     * @param size   The number of bytes from the buffer to write.
     * @hide
     * @deprecated Use {@link write(final StatsEvent statsEvent)} instead.
     *
     */
    @Deprecated
    public static void writeRaw(byte[] buffer, int size) {
        throw new RuntimeException("Stub!");
    }

    /**
     * Write an event to stats log using the raw format encapsulated in StatsEvent.
     * After writing to stats log, release() is called on the StatsEvent object.
     * No further action should be taken on the StatsEvent object following this call.
     *
     * @param statsEvent The StatsEvent object containing the encoded buffer of data to write.
     * @hide
     */
    public static void write(final StatsEvent statsEvent) {
        throw new RuntimeException("Stub!");
    }
}