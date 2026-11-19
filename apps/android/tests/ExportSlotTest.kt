package com.ocaf.builder

fun main() {
    val slot = ExportSlot()
    val first = byteArrayOf(1, 2, 3)
    check(slot.begin(first))
    first[0] = 99
    check(!slot.begin(byteArrayOf(4)))
    check(slot.take()!!.contentEquals(byteArrayOf(1, 2, 3)))
    check(slot.take() == null)
    check(slot.begin(byteArrayOf(5)))
    slot.cancel()
    check(slot.take() == null)
    check(slot.begin(byteArrayOf(6)))
    check(slot.take()!!.contentEquals(byteArrayOf(6)))
    check(!slot.begin(ByteArray(131073)))
    check(!slot.begin(ByteArray(0)))
    println("Android ExportSlot: 8 contract checks PASS")
}
