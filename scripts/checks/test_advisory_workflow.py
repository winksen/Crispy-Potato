"""Exercise the scheduled report without contacting GitHub or writing issues."""
import json
import os
import pathlib
import textwrap
import unittest
from unittest.mock import patch


class AdvisoryWorkflowTests(unittest.TestCase):
    def report(self, result, issues):
        workflow = pathlib.Path(__file__).parents[2] / '.github/workflows/ci.yml'
        code = textwrap.dedent(workflow.read_text(encoding='utf-8').rsplit('        run: |\n', 1)[1])
        with patch.dict(os.environ, {'CHECK_RESULT': result, 'RUN_URL': 'https://example.com/run'}):
            with patch('subprocess.check_output', return_value=json.dumps(issues)):
                with patch('subprocess.run') as run:
                    try:
                        exec(compile(code, str(workflow), 'exec'), {})
                    except SystemExit as exit:
                        self.assertEqual(exit.code, 0)
                    return [call.args[0] for call in run.call_args_list]

    def issue(self, state):
        return {'number': 12, 'title': 'Dependency health checks',
                'body': '<!-- crispy-potato-dependency-health -->',
                'state': state, 'author': {'login': 'app/github-actions', 'is_bot': True}}

    def test_healthy_does_not_create_an_issue(self):
        self.assertEqual(self.report('success', []), [])

    def test_failure_creates_one_issue(self):
        calls = self.report('failure', [])
        self.assertEqual(len(calls), 1)
        self.assertEqual(calls[0][:3], ['gh', 'issue', 'create'])

    def test_regression_reuses_bot_issue_by_marker(self):
        calls = self.report('failure', [self.issue('CLOSED')])
        self.assertEqual(calls[0][:4], ['gh', 'issue', 'edit', '12'])
        self.assertEqual(calls[1], ['gh', 'issue', 'reopen', '12'])

    def test_recovery_closes_existing_issue(self):
        calls = self.report('success', [self.issue('OPEN')])
        self.assertEqual(calls[1], ['gh', 'issue', 'close', '12'])

    def test_human_issue_is_not_modified(self):
        issue = self.issue('OPEN')
        issue['author']['is_bot'] = False
        calls = self.report('failure', [issue])
        self.assertEqual(calls[0][:3], ['gh', 'issue', 'create'])


if __name__ == '__main__':
    unittest.main()
