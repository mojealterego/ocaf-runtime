"""Contract tests for host-owned NOEVARIS TRS bridge client."""
import unittest
from trs_digital_life_client import BridgeClient, BridgeContractError

class BridgeClientTests(unittest.TestCase):
    def setUp(self):
        self.calls = []
        def transport(request):
            self.calls.append(request)
            return {"version": 1, "accepted": True, "payload": list(b'{"status":"ready"}')}
        self.client = BridgeClient(transport)

    def test_request_is_wire_compatible(self):
        result = self.client.call(tenant="t", sender="s", operation="trs.health", arguments={})
        self.assertEqual(result, {"status": "ready"})
        self.assertEqual(self.calls[0], {"version":1,"tenant":"t","sender":"s","operation":"trs.health","payload":[]})

    def test_no_transport_invocation_for_unauthorized_operation(self):
        with self.assertRaises(BridgeContractError):
            self.client.call(tenant="t",sender="s",operation="trs.admin",arguments={})
        self.assertEqual(self.calls, [])

    def test_rejects_response_unknown_fields(self):
        client = BridgeClient(lambda _: {"version":1,"accepted":True,"payload":[],"privileged":True})
        with self.assertRaises(BridgeContractError):
            client.call(tenant="t",sender="s",operation="trs.health",arguments={})

    def test_rejects_invalid_response_version_and_payload(self):
        for response in ({"version":2,"accepted":True,"payload":[]},
                         {"version":1,"accepted":True,"payload":[256]},
                         {"version":1,"accepted":False,"payload":[]}):
            with self.assertRaises(BridgeContractError):
                BridgeClient(lambda _: response).call(tenant="t",sender="s",operation="trs.health",arguments={})

    def test_payload_roundtrip(self):
        result = BridgeClient(lambda _: {"version":1,"accepted":True,"payload":list(b'{"distance":0.0}')}).call(
            tenant="t",sender="s",operation="trs.geometry.fisher_rao",arguments={"p":[0.5,0.5],"q":[0.5,0.5]})
        self.assertEqual(result["distance"], 0.0)

if __name__ == "__main__":
    unittest.main()
