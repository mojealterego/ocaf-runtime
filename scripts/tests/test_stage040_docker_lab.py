"""Docker process-boundary regression tests; these do not run PostgreSQL."""
from pathlib import Path
import hashlib
import json
import os
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import stage040_docker_lab as lab


IMAGE="sha256:"+"a"*64


def complete_report(*,seconds=30,witness=True):
    return {
        "schema_version":1,
        "status":"POSTGRES_3_PROCESSES_STREAMING_FAILOVER_PASSED",
        "nodes_count":3,
        **{key:True for key in lab.ha.REQUIRED_TRUE},
        "all_three_same_kernel":True,"host_power_loss_tested":False,
        "production_approved":False,"stage_040_complete":False,
        "pgbench_wal_soak":{
            "actual_pgbench_completed":True,"duration_seconds":seconds,
            "both_streaming_replicas_replayed_pgbench_dataset":True,
            "load_during_replication":True,"transactions":1000,
        },
        "witness_lab":{
            "authority":"single_local_sqlite_witness",
            "majority_consensus_tested":False,"harness_write_guard_only":True,
            "database_server_side_fencing_tested":False,
            "physical_host_fault_tested":False,
            "former_a_rejoined_by_pg_rewind":True,
            "physical_replication_slot_active_on_c":True,
            "former_a_wal_receiver_status":"streaming",
            "final_committed_event_ids":"1,2,3,4,5",
        } if witness else None,
        "full_nested_observation":{"do_not_lose":[1,2,3]},
    }


class DockerBoundary:
    def __init__(self,*,report=None,run_error=None,build_output=IMAGE,
                 cleanup_return=0,cleanup_list="",cleanup_list_return=0):
        self.report=complete_report() if report is None else report
        self.run_error=run_error
        self.build_output=build_output
        self.cleanup_return=cleanup_return
        self.cleanup_list=cleanup_list
        self.cleanup_list_return=cleanup_list_return
        self.commands=[]

    def __call__(self,argv,**kwargs):
        self.commands.append((argv,kwargs))
        self._assert_no_shell(kwargs)
        if argv[:2]==["docker","build"]:
            return subprocess.CompletedProcess(argv,0,self.build_output+"\n","")
        if argv[:2]==["docker","run"]:
            if self.run_error is not None:
                raise self.run_error
            return subprocess.CompletedProcess(argv,0,json.dumps(self.report),"")
        if argv[:2]==["docker","rm"]:
            return subprocess.CompletedProcess(argv,self.cleanup_return,"","")
        if argv[:3]==["docker","container","ls"]:
            return subprocess.CompletedProcess(argv,self.cleanup_list_return,self.cleanup_list,"")
        raise AssertionError("unexpected subprocess: "+repr(argv))

    @staticmethod
    def _assert_no_shell(kwargs):
        if kwargs.get("shell"):
            raise AssertionError("host shell must not be used")


class DockerCollectorTests(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root=Path(self.tmp.name)
        self.report=self.root/"evidence.json"
        for relative in lab.SOURCES:
            source=self.root/relative
            source.parent.mkdir(parents=True,exist_ok=True)
            source.write_text("source "+relative)

    def collect(self,boundary,**kwargs):
        with patch.object(lab.subprocess,"run",side_effect=boundary):
            return lab.run_lab(report=self.report,repo_root=self.root,**kwargs)

    def test_complete_json_captured_privately_before_tmpfs_removal(self):
        boundary=DockerBoundary()
        result=self.collect(boundary)
        self.assertEqual(result["full_nested_observation"],{"do_not_lose":[1,2,3]})
        self.assertEqual(json.loads(self.report.read_text()),result)
        collector=result["docker_collector"]
        self.assertEqual(collector["image_id"],IMAGE)
        self.assertEqual(collector["scope"],"single_host_kernel_three_postgresql_processes")
        self.assertTrue(collector["container_cleanup_verified"])
        self.assertFalse(collector["independent_hosts_verified"])
        self.assertFalse(result["stage_040_complete"])
        for relative in lab.SOURCES:
            self.assertEqual(collector["source_sha256"][relative],
                             hashlib.sha256((self.root/relative).read_bytes()).hexdigest())
        self.assertEqual(self.report.stat().st_mode & 0o777,0o600)
        self.assertEqual(self.report.stat().st_uid,os.geteuid())
        argv=boundary.commands[1][0]
        for flag in lab.RUNTIME_FLAGS:
            self.assertIn(flag,argv)
        self.assertEqual(argv[argv.index("--entrypoint")+1],"sh")
        self.assertIn(IMAGE,argv)
        self.assertIn(lab.COLLECT_COMMAND,argv)
        self.assertIn("--exercise-witness",argv)
        self.assertIn('"$@"',lab.COLLECT_COMMAND)
        self.assertEqual(boundary.commands[-1][0],
                         ["docker","rm","--force",collector["container_name"]])
        self.assertRegex(collector["container_name"],r"^ocaf040-lab-[0-9a-f]{32}$")

    def test_witness_can_be_explicitly_disabled_and_requested_duration_bound(self):
        boundary=DockerBoundary(report=complete_report(seconds=15,witness=False))
        result=self.collect(boundary,soak_seconds=15,exercise_witness=False)
        self.assertIsNone(result["witness_lab"])
        self.assertNotIn("--exercise-witness",boundary.commands[1][0])
        self.assertEqual(result["docker_collector"]["requested_soak_seconds"],15)

    def test_timeout_and_launch_error_force_remove_our_container_without_artifact(self):
        for error in (subprocess.TimeoutExpired(["docker","run"],510),OSError("launch failure")):
            boundary=DockerBoundary(run_error=error)
            with self.subTest(error=type(error).__name__),self.assertRaises(lab.DockerLabError):
                self.collect(boundary)
            self.assertEqual(boundary.commands[-1][0][:3],["docker","rm","--force"])
            run_name=boundary.commands[1][0][3]
            self.assertEqual(boundary.commands[-1][0][-1],run_name)
            self.assertFalse(self.report.exists())

    def test_bad_receipt_never_published_and_container_cleanup_still_runs(self):
        cases=[]
        for key,value in (("production_approved",True),("stage_040_complete",True),
                          ("all_three_same_kernel",False),("pgbench_wal_soak",None),
                          ("witness_lab",None)):
            cases.append({**complete_report(),key:value})
        for report in cases:
            boundary=DockerBoundary(report=report)
            with self.subTest(report=report),self.assertRaises(ValueError):
                self.collect(boundary)
            self.assertEqual(boundary.commands[-1][0][:3],["docker","rm","--force"])
            self.assertFalse(self.report.exists())

    def test_auto_removed_container_requires_verified_absence(self):
        boundary=DockerBoundary(cleanup_return=1)
        result=self.collect(boundary)
        self.assertTrue(result["docker_collector"]["container_cleanup_verified"])
        self.assertEqual(boundary.commands[-1][0][:3],["docker","container","ls"])

    def test_failed_cleanup_or_unavailable_daemon_never_publishes_success(self):
        for listing,code in (("still-running",0),("",1)):
            boundary=DockerBoundary(cleanup_return=1,cleanup_list=listing,cleanup_list_return=code)
            with self.subTest(listing=listing,code=code),self.assertRaises(lab.DockerLabError):
                self.collect(boundary)
            self.assertFalse(self.report.exists())

    def test_preexisting_destination_fails_before_build(self):
        self.report.write_text("keep prior receipt")
        boundary=DockerBoundary()
        with self.assertRaises(lab.DockerLabError):
            self.collect(boundary)
        self.assertEqual(boundary.commands,[])
        self.assertEqual(self.report.read_text(),"keep prior receipt")

    def test_invalid_controls_fail_before_build(self):
        for seconds in (0,14,121,True,"30",30.0):
            boundary=DockerBoundary()
            with self.subTest(seconds=seconds),self.assertRaises(lab.DockerLabError):
                self.collect(boundary,soak_seconds=seconds)
            self.assertEqual(boundary.commands,[])
        boundary=DockerBoundary()
        with self.assertRaises(lab.DockerLabError):
            self.collect(boundary,exercise_witness="true")
        self.assertEqual(boundary.commands,[])

    def test_immutable_image_id_required_and_cleanup_attempted_on_build_failure(self):
        boundary=DockerBoundary(build_output="latest")
        with self.assertRaises(lab.DockerLabError):
            self.collect(boundary)
        self.assertEqual([cmd[:2] for cmd,_ in boundary.commands],
                         [["docker","build"],["docker","rm"]])
        self.assertFalse(self.report.exists())

    def test_source_provenance_drift_rejects_receipt(self):
        boundary=DockerBoundary()
        actual=lab.source_hashes(self.root)
        changed={**actual,lab.SOURCES[0]:"f"*64}
        with patch.object(lab,"source_hashes",side_effect=[actual,changed]), \
             self.assertRaises(lab.DockerLabError):
            self.collect(boundary)
        self.assertFalse(self.report.exists())
