"""Stage 0.39 jailed two-tenant guest-originated UDP lab contracts."""
from pathlib import Path
import sys
import unittest
from unittest import mock
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import stage039_jailed_guest_network as jailed

class JailedGuestNetworkContracts(unittest.TestCase):
    def test_firecracker_guest_tap_is_owner_bound_and_no_router(self):
        cmd=jailed.tap_setup_command("oc39a01234","198.19.38.1",1002)
        self.assertEqual(cmd,["ip","tuntap","add","dev","oc39a01234","mode","tap","user","1002"])
        for tap in ("eth0","../tap","oc39a01234567"):
            with self.subTest(tap=tap),self.assertRaises(jailed.JailedNetError):
                jailed.tap_setup_command(tap,"198.19.38.1",1002)

    def test_jailed_config_rewrites_paths_to_private_chroot(self):
        cfg=jailed.jailed_network_config("A","oc39a01234")
        self.assertEqual(cfg["boot-source"]["kernel_image_path"],"/guest/vmlinux.bin")
        self.assertEqual(cfg["drives"][0]["path_on_host"],"/guest/rootfs.ext4")
        self.assertTrue(cfg["drives"][0]["is_read_only"])
        self.assertEqual(cfg["network-interfaces"][0]["host_dev_name"],"oc39a01234")
        with self.assertRaises(jailed.JailedNetError):
            jailed.jailed_network_config("A","oc39b01234")

    def test_two_real_processes_must_have_separate_kernel_cgroup_inodes(self):
        a={"vm_id":"ocaf-"+"a"*24,"tenant_id":"tenant-a","cgroup_inode":1001,
           "guest_kernel_console_seen":True,"firecracker_api_running":True,
           "process_in_expected_cgroup":True}
        b={**a,"vm_id":"ocaf-"+"b"*24,"tenant_id":"tenant-b","cgroup_inode":1002}
        self.assertTrue(jailed.verify_pair(a,b)["two_live_jailer_tenants"])
        for changes in ({"cgroup_inode":1001},{"vm_id":a["vm_id"]},
                        {"tenant_id":"tenant-a"},{"firecracker_api_running":False}):
            with self.subTest(changes=changes),self.assertRaises(jailed.JailedNetError):
                jailed.verify_pair(a,{**b,**changes})

    def test_late_pid_and_cgroup_changes_are_rejected(self):
        p=mock.Mock(pid=42424);p.poll.return_value=None
        with mock.patch.object(jailed.fc.vm_egress,"require_vm_running",return_value=909):
            jailed.assert_live(p,"ocaf-"+"a"*24,Path("/sys/fs/cgroup/ocaf-stage039-vm-"+"a"*16+".service"),909)
            with self.assertRaises(jailed.JailedNetError):
                jailed.assert_live(p,"ocaf-"+"a"*24,Path("/sys/fs/cgroup/ocaf-stage039-vm-"+"a"*16+".service"),910)
