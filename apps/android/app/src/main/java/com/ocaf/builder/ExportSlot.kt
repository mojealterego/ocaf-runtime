package com.ocaf.builder

/** Own one SAF export payload until its activity result; never replace a pending export. */
internal class ExportSlot {
    private var pending: ByteArray? = null

    @Synchronized
    fun begin(bytes: ByteArray): Boolean {
        if (pending != null || bytes.isEmpty() || bytes.size > 128 * 1024) return false
        pending = bytes.copyOf()
        return true
    }

    @Synchronized
    fun take(): ByteArray? {
        val result = pending
        pending = null
        return result
    }

    @Synchronized
    fun cancel() {
        pending?.fill(0)
        pending = null
    }
}
