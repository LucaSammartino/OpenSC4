import suite_triage


def test_no_load_after_clock_is_a_launcher_failure():
    row = suite_triage.diagnose('autotest: clock started; verdict in 240 s\n',
                               'verdict: no-load\nline: no city load within 300 s\n', '2028-01-01')
    assert row['cause'] == 'launcher-load-detection'
    assert row['verdict'] == 'no-load'
    assert row['address'] == '-'


def test_lua_launch_rejection_is_not_a_game_failure():
    row = suite_triage.diagnose('', 'verdict: unknown\nline: no verdict line\n'
                               'lua_script must name an existing repo-relative .lua file', '2000-03-01')
    assert row['cause'] == 'launcher-lua-checkout'
    assert row['address'] == '-'


def test_stall_preserves_first_sample_and_pc():
    log = ('WATCHDOG: main thread (tid 42) stalled - no OnTick for 5100 ms\n'
           '  context: eip=0x7724915C (ntdll.dll+0x7915C)\n'
           'WATCHDOG: main thread (tid 42) stalled - no OnTick for 10000 ms\n')
    row = suite_triage.diagnose(log, 'verdict: stalled\nline: watchdog\n', '2028-01-01')
    assert row['address'] == '0x7724915C'
    assert '5100 ms' in row['first_exception_or_stall']
    assert '10000 ms' not in row['first_exception_or_stall']


def test_access_violation_without_callback_does_not_invent_an_address():
    row = suite_triage.diagnose('script: line 6 2000-01-02 terrain -> ok\n',
                               'verdict: exited\nline: exited (code -1073741819) without a verdict\n',
                               '2000-04-01')
    assert row['cause'] == 'unhandled-exit-no-address'
    assert row['address'] == '-'
    assert 'terrain' in row['first_exception_or_stall']


def test_exhausted_window_is_distinct_from_unestablished_city():
    row = suite_triage.diagnose('AUTOTEST VERDICT: clean (date=2027-02-23, days_per_second=0.52)',
                               'verdict: clean\n', '2028-01-28')
    assert row['cause'] == 'window-exhausted'
    row = suite_triage.diagnose('AUTOTEST VERDICT: clean (date=2000-01-01, days_per_second=0.00)',
                               'verdict: clean\n', '2000-04-01')
    assert row['cause'] == 'unestablished-city'
