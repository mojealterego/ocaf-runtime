package com.ocaf.builder

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import uniffi.ocaf_builder_mobile_ffi.validateLocalJsonDsl

/** Runs the generated Kotlin binding against the ABI actually installed on Android. */
@RunWith(AndroidJUnit4::class)
class NativeBuilderFfiTest {
    private val source = """{"schema_version":1,"tenant_id":"local","project_id":"native_test","revision":1,"max_cost_minor":0,"grants":[],"nodes":[{"id":"input","kind":"input","inputs":[],"outputs":[{"name":"out","kind":"text"}],"tool_name":null,"max_cost_minor":0},{"id":"output","kind":"output","inputs":[{"name":"in","kind":"text"}],"outputs":[],"tool_name":null,"max_cost_minor":0}],"edges":[{"from_node":"input","from_port":"out","to_node":"output","to_port":"in"}]}"""

    @Test fun rustPlanIsDeterministicAndInert() {
        val first = validateLocalJsonDsl(source)
        val second = validateLocalJsonDsl(source)
        assertEquals(first, second)
        val plan = JSONObject(first)
        assertEquals("local", plan.getString("namespace"))
        assertFalse(plan.getBoolean("effects_executed"))
        assertTrue(plan.getString("project_sha256").matches(Regex("[0-9a-f]{64}")))
        assertEquals("input", plan.getJSONArray("ordered_nodes").getString(0))
        assertEquals("output", plan.getJSONArray("ordered_nodes").getString(1))
    }

    @Test fun crossTenantGraphRejected() {
        assertTrue(runCatching { validateLocalJsonDsl(source.replace("\"tenant_id\":\"local\"", "\"tenant_id\":\"foreign\"")) }.isFailure)
    }
}
