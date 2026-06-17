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

public final class StatsEvent {
    public static final byte TYPE_INT = 0x00;
    public static final byte TYPE_LONG = 0x01;
    public static final byte TYPE_STRING = 0x02;
    public static final byte TYPE_LIST = 0x03;
    public static final byte TYPE_FLOAT = 0x04;
    public static final byte TYPE_BOOLEAN = 0x05;
    public static final byte TYPE_BYTE_ARRAY = 0x06;
    public static final byte TYPE_OBJECT = 0x07;
    public static final byte TYPE_KEY_VALUE_PAIRS = 0x08;
    public static final byte TYPE_ATTRIBUTION_CHAIN = 0x09;
    public static final byte TYPE_ERRORS = 0x0F;
    public static final int ERROR_NO_TIMESTAMP = 0x1;
    public static final int ERROR_NO_ATOM_ID = 0x2;
    public static final int ERROR_OVERFLOW = 0x4;
    public static final int ERROR_ATTRIBUTION_CHAIN_TOO_LONG = 0x8;
    public static final int ERROR_TOO_MANY_KEY_VALUE_PAIRS = 0x10;
    public static final int ERROR_ANNOTATION_DOES_NOT_FOLLOW_FIELD = 0x20;
    public static final int ERROR_INVALID_ANNOTATION_ID = 0x40;
    public static final int ERROR_ANNOTATION_ID_TOO_LARGE = 0x80;
    public static final int ERROR_TOO_MANY_ANNOTATIONS = 0x100;
    public static final int ERROR_TOO_MANY_FIELDS = 0x200;
    public static final int ERROR_ATTRIBUTION_UIDS_TAGS_SIZES_NOT_EQUAL = 0x1000;
    public static final int ERROR_ATOM_ID_INVALID_POSITION = 0x2000;
    public static final int ERROR_LIST_TOO_LONG = 0x4000;

    public static final int MAX_ANNOTATION_COUNT = 15;
    public static final int MAX_ATTRIBUTION_NODES = 127;
    public static final int MAX_NUM_ELEMENTS = 127;
    public static final int MAX_KEY_VALUE_PAIRS = 127;

    private StatsEvent() {
        throw new RuntimeException("Stub!");
    }

    public static StatsEvent.Builder newBuilder() {
        throw new RuntimeException("Stub!");
    }

    public int getAtomId() {
        throw new RuntimeException("Stub!");
    }

    public byte[] getBytes() {
        throw new RuntimeException("Stub!");
    }

    public int getNumBytes() {
        throw new RuntimeException("Stub!");
    }

    public void release() {
        throw new RuntimeException("Stub!");
    }

    public static final class Builder {
        private Builder() {
            throw new RuntimeException("Stub!");
        }

        public Builder setAtomId(final int atomId) {
            throw new RuntimeException("Stub!");
        }

        public Builder writeBoolean(final boolean value) {
            throw new RuntimeException("Stub!");
        }

        public Builder writeInt(final int value) {
            throw new RuntimeException("Stub!");
        }

        public Builder writeLong(final long value) {
            throw new RuntimeException("Stub!");
        }

        public Builder writeFloat(final float value) {
            throw new RuntimeException("Stub!");
        }

        public Builder writeString(final String value) {
            throw new RuntimeException("Stub!");
        }

        public Builder writeByteArray(final byte[] value) {
            throw new RuntimeException("Stub!");
        }

        public Builder writeAttributionChain(final int[] uids, final String[] tags) {
            throw new RuntimeException("Stub!");
        }

        /* @NonNull
        public Builder writeKeyValuePairs(
            @Nullable final SparseIntArray intMap,
            @Nullable final SparseLongArray longMap,
            @Nullable final SparseArray<String> stringMap,
            @Nullable final SparseArray<Float> floatMap) {
            throw new RuntimeException("Stub!");
        } */

        public Builder writeBooleanArray(final boolean[] elements) {
            throw new RuntimeException("Stub!");
        }

        public Builder writeIntArray(final int[] elements) {
            throw new RuntimeException("Stub!");
        }

        public Builder writeLongArray(final long[] elements) {
            throw new RuntimeException("Stub!");
        }

        public Builder writeFloatArray(final float[] elements) {
            throw new RuntimeException("Stub!");
        }

        public Builder writeStringArray(final String[] elements) {
            throw new RuntimeException("Stub!");
        }

        public Builder addBooleanAnnotation(final byte annotationId, final boolean value) {
            throw new RuntimeException("Stub!");
        }

        public Builder addIntAnnotation(final byte annotationId, final int value) {
            throw new RuntimeException("Stub!");
        }

        public Builder usePooledBuffer() {
            throw new RuntimeException("Stub!");
        }

        public StatsEvent build() {
            throw new RuntimeException("Stub!");
        }
    }
}