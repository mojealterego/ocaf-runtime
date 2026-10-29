import unittest
from scripts.stage042_release_decision import evaluate, WORKFLOWS

SHA='a'*40
REPO='mojealterego/ocaf-runtime'

def runs():
    return {label:dict(id=i,run_attempt=1,head_sha=SHA,path=path,
                      head_repository={'full_name':REPO},repository={'full_name':REPO},
                      head_branch='main',event='push',status='completed',conclusion='success')
            for i,(label,path) in enumerate(WORKFLOWS.items(),1)}

class Decision(unittest.TestCase):
    def test_passing_ci_does_not_fabricate_acceptance(self):
        result=evaluate(SHA,REPO,runs())
        self.assertEqual(result['status'],'NO_GO')
        self.assertFalse(result['production_approved'])
        self.assertEqual(len(result['ci']),4)
    def test_wrong_commit_repository_or_workflow_rejected(self):
        for change in ({'head_sha':'b'*40},{'path':WORKFLOWS['rollback']},
                       {'head_repository':{'full_name':'other/repo'}},
                       {'event':'pull_request'},{'head_branch':'feature'}):
            data=runs(); data['workspace'].update(change)
            with self.assertRaises(ValueError): evaluate(SHA,REPO,data)
    def test_missing_pending_failed_runs_are_blockers(self):
        data=runs();data['rollback']=None
        data['workspace']['status']='in_progress'
        data['supply_chain']['conclusion']='failure'
        result=evaluate(SHA,REPO,data)
        for reason in ('rollback:missing_run','workspace:not_successful','supply_chain:not_successful'):
            self.assertIn(reason,result['blockers'])
