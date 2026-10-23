"""Regression tests using gh's JSON shape and app-prefixed bot login."""
import json
import pathlib
import tempfile
import textwrap
import unittest
from unittest.mock import patch

import weekly


class WeeklyTests(unittest.TestCase):
    def test_marker_finds_existing_app_prefixed_bot_and_reopens_it(self):
        fixture = pathlib.Path(__file__).parent / 'fixtures/gh-health-issues.json'
        with patch('weekly.subprocess.check_output', return_value=fixture.read_text()):
            with patch('weekly.subprocess.run') as run:
                with tempfile.TemporaryDirectory() as directory:
                    weekly.main([str(pathlib.Path(directory) / 'missing.json')])
        calls = [call.args[0] for call in run.call_args_list]
        self.assertEqual(calls[0][:4], ['gh', 'issue', 'edit', '17'])
        self.assertEqual(calls[1], ['gh', 'issue', 'reopen', '17'])
        self.assertFalse(any('create' in call for call in calls))

    def test_human_issue_cannot_impersonate_reporter(self):
        issues = [{'number': 3, 'title': weekly.TITLE, 'body': weekly.MARKER,
                   'state': 'OPEN', 'author': {'is_bot': False}}]
        with patch('weekly.subprocess.check_output', return_value=json.dumps(issues)):
            with patch('weekly.subprocess.run') as run:
                with tempfile.TemporaryDirectory() as directory:
                    weekly.main([str(pathlib.Path(directory) / 'missing.json')])
        self.assertEqual(run.call_args.args[0][:3], ['gh', 'issue', 'create'])

    def test_healthy_reports_close_existing_issue(self):
        issues = [{'number': 17, 'title': weekly.TITLE, 'body': weekly.MARKER,
                   'state': 'OPEN', 'author': {'is_bot': True}}]
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / 'report.json'
            path.write_text('{"problems": [], "skipped": []}')
            with patch('weekly.subprocess.check_output', return_value=json.dumps(issues)):
                with patch('weekly.subprocess.run') as run:
                    weekly.main([str(path)])
        self.assertEqual(run.call_args.args[0], ['gh', 'issue', 'close', '17'])

    def test_bad_or_oversized_reports_become_visible_failures(self):
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / 'report.json'
            for content in ['{}', '{"problems": [null]}', 'x' * (1024 * 1024 + 1)]:
                path.write_text(content)
                body, attention = weekly.summary([str(path)])
                self.assertTrue(attention)
                self.assertIn('Check failed before producing a valid report', body)

    def test_inline_reporter_matches_tested_script(self):
        root = pathlib.Path(__file__).parents[2]
        workflow = (root / '.github/workflows/content-health.yml').read_text()
        inline = textwrap.dedent(workflow.rsplit('        run: |\n', 1)[1])
        source = (root / 'scripts/checks/weekly.py').read_text().split('if __name__ == "__main__":')[0].rstrip()
        expected = source + '\n\nmain(["reports/links/link-report.json", "reports/identifiers/id-report.json"])\n'
        self.assertEqual(inline, expected)
        compile(inline, 'workflow reporter', 'exec')
        report_job = workflow.split('  report:\n')[1]
        self.assertNotIn('actions/checkout', report_job)
        self.assertNotIn('secrets.', report_job)
        self.assertIn('      issues: write', report_job)
        self.assertNotIn('issues: write', workflow.split('  report:\n')[0])


if __name__ == '__main__':
    unittest.main()
