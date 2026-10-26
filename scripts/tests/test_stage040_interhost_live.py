"""Stage 0.40.3 live remote-host PostgreSQL WAL acceptance requires REAL SSH host evidence."""
from pathlib import Path
import sys
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import stage040_interhost_live as pg

def evidence():
    return {
        "schema_version":1,
        "status":"REAL_DISTINCT_LINUX_KERNELS_POSTGRESQL_LIVE_TCP_STREAMING_VERIFIED",
        "topology":"three_truly_distinct_linux_kernels",
        "boot_ids":{"a":"11111111-1111-4111-8111-111111111111",
                    "b":"22222222-2222-4222-8222-222222222222",
                    "c":"33333333-3333-4333-8333-333333333333"},
        "boot_ids_after":{"a":"11111111-1111-4111-8111-111111111111",
                          "b":"22222222-2222-4222-8222-222222222222",
                          "c":"33333333-3333-4333-8333-333333333333"},
        "client_addresses":["10.60.0.2","10.60.0.3"],
        "client_addresses_after":["10.60.0.2","10.60.0.3"],
        "standby_addresses":{"b":"10.60.0.2","c":"10.60.0.3"},
        "source_replication_address":"10.60.0.1",
        "postgres_port":5540,
        "receiver_snapshots":{
            phase:{role:{"status":"streaming","sender_host":"10.60.0.1",
                         "sender_port":5540,"written_lsn":"0/300"}
                   for role in ("b","c")}
            for phase in ("before","after")},
        "source_writable":True,"b_in_recovery":True,"c_in_recovery":True,
        "replication_b_status":"streaming","replication_c_status":"streaming",
        "source_nonce":"a"*32,"standby_b_nonce":"a"*32,"standby_c_nonce":"a"*32,
        "standby_b_origin_boot_id":"11111111-1111-4111-8111-111111111111",
        "standby_c_origin_boot_id":"11111111-1111-4111-8111-111111111111",
        "source_wal_lsn":"0/16D4D20",
        "standby_b_wal_replay_lsn":"0/16D4D20",
        "standby_c_wal_replay_lsn":"0/16D4D20",
        "standby_b_wal_replay_at_or_above_source_lsn":True,
        "standby_c_wal_replay_at_or_above_source_lsn":True,
        "used_ssh_host_key_pinning":True,
        "actual_direct_network_replication":True,
        "physical_host_power_off_tested":False,
        "consensus_fencing_verified":False,
        "production_approved":False,
        "stage_040_complete":False,
    }

class RealIndependentHostPgTests(unittest.TestCase):
    def test_positive_requires_three_actual_unique_host_boot_ids_and_direct_network(self):
        report=pg.validate_receipt(evidence())
        self.assertEqual(report["status"],"REAL_DISTINCT_LINUX_KERNELS_POSTGRESQL_LIVE_TCP_STREAMING_VERIFIED")
        self.assertTrue(report["actual_direct_network_replication"])
        self.assertFalse(report["consensus_fencing_verified"])

    def test_rejects_same_host_local_netns_or_artifact_faked_tcp(self):
        valid=evidence()
        mods=[
            {"boot_ids":{**valid["boot_ids"],"c":valid["boot_ids"]["b"]}},
            {"topology":"three_netns_one_kernel"},
            {"actual_direct_network_replication":False},
            {"consensus_fencing_verified":True},
            {"production_approved":True},
            {"physical_host_power_off_tested":True},
            {"client_addresses":["10.60.0.2","10.60.0.2"]},
            {"standby_b_nonce":"f"*32},
            {"standby_b_origin_boot_id":valid["boot_ids"]["b"]},
            {"standby_c_wal_replay_at_or_above_source_lsn":False},
            {"replication_c_status":"stopped"},
        ]
        for change in mods:
            with self.subTest(change=change),self.assertRaises(pg.InterhostPgError):
                pg.validate_receipt({**valid,**change})

    def test_invalid_host_targets_and_malformed_ips_rejected(self):
        valid=pg.validate_host({"role":"a","host":"10.60.0.1","ssh_user":"pgops",
                                "ssh_port":22,"replication_address":"10.60.0.1"})
        self.assertEqual(valid["host"],"10.60.0.1")
        for data in (
            {"host":"127.0.0.1"},{"host":"10.60.0.1;rm -rf /"},
            {"ssh_user":"root;id"},{"ssh_port":0},
            {"replication_address":"10.60.0.1x"},
        ):
            with self.subTest(data=data),self.assertRaises(pg.InterhostPgError):
                pg.validate_host({**valid,**data})

    def test_dangerous_sql_and_non_private_pinned_host_file_rejected(self):
        for nonce in ("x","' OR 1=1","a"*33):
            with self.subTest(nonce=nonce),self.assertRaises(pg.InterhostPgError):
                pg.safe_nonce(nonce)

# These are injected-command regression tests. They do not observe real SSH
# hosts and must never be submitted as interhost deployment evidence.
from contextlib import nullcontext
import json
import os
import tempfile
from unittest.mock import patch


def inventory():
    return {
        "schema_version":1,"database":"ocaf_stage040_lab",
        "postgres_port":5540,"postgres_socket_dir":"/var/run/postgresql",
        "nodes":{role:{"role":role,"host":f"10.60.0.{i}",
                       "replication_address":f"10.60.0.{i}",
                       "ssh_port":22,"ssh_user":"pgops"}
                 for i,role in enumerate(pg.ROLES,1)},
    }


class InjectedSSH:
    """Model exact observed command outputs, including delayed DDL replay."""
    def __init__(self,*,fault=None):
        self.fault=fault
        self.boot_calls={r:0 for r in pg.ROLES}
        self.role_calls={r:0 for r in pg.ROLES}
        self.receiver_calls={r:0 for r in ("b","c")}
        self.replay_calls={r:0 for r in ("b","c")}
        self.clients_calls=0
        self.table_calls=[]
        self.commands=[]

    def __call__(self,node,known_hosts,argv,**kwargs):
        role=node["role"]
        self.commands.append((role,argv))
        if argv==["cat","/proc/sys/kernel/random/boot_id"]:
            self.boot_calls[role]+=1
            if self.fault=="reboot" and role=="c" and self.boot_calls[role]>1:
                return "44444444-4444-4444-8444-444444444444"
            if self.fault=="same_kernel" and role=="c":
                return evidence()["boot_ids"]["b"]
            return evidence()["boot_ids"][role]
        query=argv[-1]
        if query=="SELECT pg_is_in_recovery()":
            self.role_calls[role]+=1
            if self.fault=="promoted" and role=="b" and self.role_calls[role]>1:
                return "f"
            return "f" if role=="a" else "t"
        if query.startswith("SELECT client_addr"):
            self.clients_calls+=1
            if self.fault=="client_changed" and self.clients_calls>1:
                return "10.60.0.2\n10.60.0.4"
            return "10.60.0.2\n10.60.0.3"
        if query.startswith("SELECT row_to_json(receiver)"):
            self.receiver_calls[role]+=1
            row={"status":"streaming","sender_host":"10.60.0.1",
                 "sender_port":5540,"written_lsn":"0/300",
                 "flushed_lsn":"0/300","latest_end_lsn":"0/300",
                 "conninfo":"password=never-persist-this"}
            if self.fault=="wrong_sender":
                row["sender_host"]="10.60.0.9"
            if self.fault=="wrong_port":
                row["sender_port"]=5432
            if self.fault=="receiver_stopped" and self.receiver_calls[role]>1:
                row["status"]="stopped"
            if self.fault=="malformed_receiver_lsn":
                row["written_lsn"]="100000000/1"
            if self.fault=="multiple_receivers":
                return json.dumps(row)+"\n"+json.dumps(row)
            return json.dumps(row)
        if query.startswith("CREATE TABLE") or query.startswith("INSERT INTO"):
            return ""
        if query=="SELECT pg_current_wal_lsn()":
            return "0/200" if self.fault!="malformed_source_lsn" else "1/100000000"
        if query=="SELECT pg_last_wal_replay_lsn()":
            self.replay_calls[role]+=1
            if self.fault=="never_replayed":
                return "0/100"
            return ("","0/100","0/300")[min(self.replay_calls[role]-1,2)]
        if query.startswith("SELECT nonce || ':' || origin_boot_id"):
            if self.replay_calls[role]<3:
                raise AssertionError("DDL table queried before its WAL was replayed")
            self.table_calls.append(role)
            if self.fault=="sql_error":
                raise pg.InterhostPgError("remote psql error")
            nonce="b"*32 if self.fault=="wrong_nonce" else "a"*32
            return nonce+":"+evidence()["boot_ids"]["a"]
        raise AssertionError("unexpected command: "+repr(argv))


class InjectedInterhostHarnessTests(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.directory=Path(self.tmp.name)
        self.known=self.directory/"known_hosts"
        self.known.write_text("\n".join(
            f"10.60.0.{i} ssh-ed25519 "+"A"*60 for i in (1,2,3)))
        self.known.chmod(0o600)
        self.report=self.directory/"report.json"

    def run_injected(self,probe,*,clock=None):
        clock_patch=(patch.object(pg.time,"monotonic",side_effect=clock)
                     if clock is not None else nullcontext())
        with patch.object(pg.secrets,"token_hex",return_value="a"*32), \
             patch.object(pg.time,"sleep"), clock_patch:
            return pg.run_lab(inventory(),self.known,self.report,ssh=probe)

    def test_complete_injected_harness_waits_for_ddl_and_records_raw_observations(self):
        probe=InjectedSSH()
        result=self.run_injected(probe)
        self.assertEqual(probe.table_calls,["b","c"])
        self.assertEqual(probe.boot_calls,{"a":2,"b":2,"c":2})
        self.assertEqual(probe.receiver_calls,{"b":2,"c":2})
        self.assertEqual(result["standby_b_wal_replay_lsn"],"0/300")
        self.assertEqual(result["receiver_snapshots"]["after"]["b"]["sender_host"],"10.60.0.1")
        self.assertEqual(result["receiver_snapshots"]["before"]["c"]["written_lsn"],"0/300")
        self.assertNotIn("conninfo",result["receiver_snapshots"]["after"]["b"])
        self.assertFalse(result["stage_040_complete"])
        self.assertFalse(result["production_approved"])
        self.assertEqual(json.loads(self.report.read_text()),result)
        self.assertEqual(os.stat(self.report).st_mode & 0o777,0o600)

    def test_fails_without_receipt_for_wrong_primary_reboot_stopped_receiver_or_sql_error(self):
        for fault in ("same_kernel","wrong_sender","wrong_port","reboot","receiver_stopped",
                      "client_changed","promoted","malformed_receiver_lsn",
                      "malformed_source_lsn","multiple_receivers","sql_error"):
            with self.subTest(fault=fault),self.assertRaises(pg.InterhostPgError):
                self.run_injected(InjectedSSH(fault=fault))
            self.assertFalse(self.report.exists())

    def test_timeout_does_not_claim_unobserved_nonce_or_wal_replay(self):
        for fault in ("never_replayed","wrong_nonce"):
            with self.subTest(fault=fault),self.assertRaises(pg.InterhostPgError):
                self.run_injected(InjectedSSH(fault=fault),clock=iter(range(0,1000,5)))
            self.assertFalse(self.report.exists())

    def test_existing_report_is_never_overwritten(self):
        self.report.write_text("previous evidence")
        probe=InjectedSSH()
        with self.assertRaises(pg.InterhostPgError):
            self.run_injected(probe)
        self.assertEqual(self.report.read_text(),"previous evidence")
        self.assertEqual(probe.commands,[])

    def test_numeric_lsn_bounds_and_raw_receipt_consistency(self):
        self.assertEqual(pg.lsn_position("FFFFFFFF/FFFFFFFF"),2**64-1)
        for value in ("100000000/0","0/100000000","-1/1","0/0\n",None,1):
            with self.subTest(value=value),self.assertRaises(pg.InterhostPgError):
                pg.lsn_position(value)
        valid=evidence()
        for change in (
            {"source_wal_lsn":"0/0"},
            {"source_wal_lsn":"100000000/0"},
            {"standby_b_wal_replay_lsn":"0/1"},
            {"boot_ids_after":{**valid["boot_ids"],"c":valid["boot_ids"]["b"]}},
            {"client_addresses_after":["10.60.0.2","10.60.0.9"]},
            {"receiver_snapshots":{}},
            {"source_replication_address":"10.60.0.2"},
        ):
            with self.subTest(change=change),self.assertRaises(pg.InterhostPgError):
                pg.validate_receipt({**valid,**change})
        for missing in ("boot_ids_after","client_addresses_after","receiver_snapshots",
                        "source_replication_address","standby_b_wal_replay_lsn",
                        "standby_b_origin_boot_id"):
            with self.subTest(missing=missing),self.assertRaises(pg.InterhostPgError):
                pg.validate_receipt({key:value for key,value in valid.items() if key!=missing})

    def test_inventory_requires_private_static_host_key_pins(self):
        self.known.chmod(0o644)
        with self.assertRaises(pg.InterhostPgError):
            pg.validate_inventory(inventory(),self.known)
        self.known.chmod(0o600)
        self.known.write_text("10.60.0.1 ssh-ed25519 "+"A"*100)
        with self.assertRaises(pg.InterhostPgError):
            pg.validate_inventory(inventory(),self.known)

    def test_ssh_transport_enforces_host_key_pinning_and_fails_on_probe_error(self):
        node=inventory()["nodes"]["a"]
        completed=pg.subprocess.CompletedProcess([],0,"observed\n","")
        with patch.object(pg.subprocess,"run",return_value=completed) as command:
            self.assertEqual(pg._ssh(node,self.known,["cat","/proc/sys/kernel/random/boot_id"]),"observed")
        argv=command.call_args.args[0]
        self.assertIn("StrictHostKeyChecking=yes",argv)
        self.assertIn("BatchMode=yes",argv)
        self.assertIn("IdentitiesOnly=yes",argv)
        self.assertIn("UserKnownHostsFile="+str(self.known),argv)
        for failed in (pg.subprocess.CompletedProcess([],1,"", "error"),
                       pg.subprocess.CompletedProcess([],0,"x"*8193,"")):
            with patch.object(pg.subprocess,"run",return_value=failed),self.assertRaises(pg.InterhostPgError):
                pg._ssh(node,self.known,["cat","/proc/sys/kernel/random/boot_id"])
