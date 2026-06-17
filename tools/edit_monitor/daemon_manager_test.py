# Copyright 2024, The Android Open Source Project
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Unittests for DaemonManager."""

import fcntl
import hashlib
import logging
import multiprocessing
import os
import pathlib
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from unittest import mock
from edit_monitor import daemon_manager
from proto import edit_event_pb2


TEST_BINARY_FILE = '/path/to/test_binary'
TEST_PID_FILE_PATH = (
    '587239c2d1050afdf54512e2d799f3b929f86b43575eb3c7b4bab105dd9bd25e.lock'
)


def simple_daemon(output_file):
  with open(output_file, 'w') as f:
    f.write('running daemon target')


def long_running_daemon():
  while True:
    time.sleep(1)


def memory_consume_daemon_target(size_mb):
  try:
    size_bytes = size_mb * 1024 * 1024
    dummy_data = bytearray(size_bytes)
    time.sleep(10)
  except MemoryError:
    print(f'Process failed to allocate {size_mb} MB of memory.')


def cpu_consume_daemon_target(target_usage_percent):
  while True:
    start_time = time.time()
    while time.time() - start_time < target_usage_percent / 100:
      pass  # Busy loop to consume CPU

    # Sleep to reduce CPU usage
    time.sleep(1 - target_usage_percent / 100)


class DaemonManagerTest(unittest.TestCase):

  @classmethod
  def setUpClass(cls):
    super().setUpClass()
    # Configure to print logging to stdout.
    logging.basicConfig(filename=None, level=logging.DEBUG)
    console = logging.StreamHandler(sys.stdout)
    logging.getLogger('').addHandler(console)

  def setUp(self):
    super().setUp()
    self.original_tempdir = tempfile.tempdir
    self.working_dir = tempfile.TemporaryDirectory()
    # Sets the tempdir under the working dir so any temp files created during
    # tests will be cleaned.
    tempfile.tempdir = self.working_dir.name
    self.patch = mock.patch.dict(
        os.environ, {'ENABLE_ANDROID_EDIT_MONITOR': 'true'}
    )
    self.patch.start()

  def tearDown(self):
    # Cleans up any child processes left by the tests.
    self._cleanup_child_processes()
    self.working_dir.cleanup()
    # Restores tempdir.
    tempfile.tempdir = self.original_tempdir
    self.patch.stop()
    super().tearDown()

  def test_start_success_with_no_existing_instance(self):
    self.assert_run_simple_daemon_success()

  def test_start_success_with_existing_instance_running(self):
    # Create a running daemon subprocess
    p = self._create_fake_deamon_process()

    self.assert_run_simple_daemon_success()
    self.assert_no_subprocess_running()

  def test_start_success_with_existing_instance_already_dead(self):
    # Create a pidfile with pid that does not exist.
    pid_file_path_dir = pathlib.Path(self.working_dir.name).joinpath(
        'edit_monitor'
    )
    pid_file_path_dir.mkdir(parents=True, exist_ok=True)
    with open(pid_file_path_dir.joinpath(TEST_PID_FILE_PATH), 'w') as f:
      f.write('123456')

    self.assert_run_simple_daemon_success()

  def test_start_success_with_existing_instance_from_different_binary(self):
    # First start an instance based on "some_binary_path"
    existing_dm = daemon_manager.DaemonManager(
        'some_binary_path',
        daemon_target=long_running_daemon,
    )
    existing_dm.start()

    self.assert_run_simple_daemon_success()
    existing_dm.stop()

  def test_start_success_with_chrome_target_repo(self):
    dm = daemon_manager.DaemonManager(
        TEST_BINARY_FILE,
        daemon_target=simple_daemon,
        daemon_args=(
            tempfile.NamedTemporaryFile(dir=self.working_dir.name).name,),
        target_repo='chrome',
    )
    dm.start()
    dm.monitor_daemon(interval=1)

    # Verify the expected pid file is created based on the chrome key.
    hash_object = hashlib.sha256()
    hash_object.update('edit_monitor_chrome'.encode('utf-8'))
    expected_pid_file_path = pathlib.Path(self.working_dir.name).joinpath(
        'edit_monitor', hash_object.hexdigest() + '.lock'
    )
    self.assertTrue(expected_pid_file_path.exists())

    dm.stop()

  def test_start_chrome_monitor_kills_existing_instance_from_different_binary(
      self,
  ):
    # Calculate the pid file path for chrome.
    hash_object = hashlib.sha256()
    hash_object.update('edit_monitor_chrome'.encode('utf-8'))
    chrome_pid_file_name = hash_object.hexdigest() + '.lock'

    # First start an instance based on "binary_path_1"
    # We use a fake process to simulate the existing instance to avoid
    # the race condition that the same process (test process) is used for
    # both instances.
    p = self._create_fake_deamon_process(name=chrome_pid_file_name)

    # Then start another instance based on "binary_path_2"
    dm = daemon_manager.DaemonManager(
        'binary_path_2',
        daemon_target=long_running_daemon,
        target_repo='chrome',
    )
    dm.start()

    # Verify that the first instance is killed and the second instance is alive.
    self.assertFalse(p.is_alive())
    self.assertTrue(dm.daemon_process.is_alive())

    dm.stop()

  def test_start_return_directly_if_block_sign_exists(self):
    # Creates the block sign.
    pathlib.Path(self.working_dir.name).joinpath(
        daemon_manager.BLOCK_SIGN_FILE
    ).touch()

    dm = daemon_manager.DaemonManager(TEST_BINARY_FILE)
    dm.start()

    # Verify no daemon process is started.
    self.assertIsNone(dm.daemon_process)

  @mock.patch.dict(
      os.environ, {'ENABLE_ANDROID_EDIT_MONITOR': 'false'}, clear=True
  )
  def test_start_return_directly_if_disabled(self):
    dm = daemon_manager.DaemonManager(TEST_BINARY_FILE)
    dm.start()

    # Verify no daemon process is started.
    self.assertIsNone(dm.daemon_process)

  def test_start_return_directly_if_in_cog_env(self):
    dm = daemon_manager.DaemonManager(
        '/google/cog/cloud/user/workspace/edit_monitor'
    )
    dm.start()

    # Verify no daemon process is started.
    self.assertIsNone(dm.daemon_process)

  def test_start_failed_other_instance_is_starting(self):
    f = open(
        pathlib.Path(self.working_dir.name).joinpath(
            TEST_PID_FILE_PATH + '.setup'
        ),
        'w',
    )
    # Acquire an exclusive lock
    fcntl.flock(f, fcntl.LOCK_EX | fcntl.LOCK_NB)

    dm = daemon_manager.DaemonManager(TEST_BINARY_FILE)
    dm.start()

    # Release the lock
    fcntl.flock(f, fcntl.LOCK_UN)
    f.close()
    # Verify no daemon process is started.
    self.assertIsNone(dm.daemon_process)

  @mock.patch('os.kill')
  def test_start_failed_to_kill_existing_instance(self, mock_kill):
    mock_kill.side_effect = OSError('Unknown OSError')
    pid_file_path_dir = pathlib.Path(self.working_dir.name).joinpath(
        'edit_monitor'
    )
    pid_file_path_dir.mkdir(parents=True, exist_ok=True)
    with open(pid_file_path_dir.joinpath(TEST_PID_FILE_PATH), 'w') as f:
      f.write('123456')

    fake_cclient = FakeClearcutClient()
    with self.assertRaises(OSError):
      dm = daemon_manager.DaemonManager(TEST_BINARY_FILE, cclient=fake_cclient)
      dm.start()
    self._assert_error_event_logged(
        fake_cclient, edit_event_pb2.EditEvent.FAILED_TO_START_EDIT_MONITOR
    )

  def test_start_failed_to_write_pidfile(self):
    pid_file_path_dir = pathlib.Path(self.working_dir.name).joinpath(
        'edit_monitor'
    )
    pid_file_path_dir.mkdir(parents=True, exist_ok=True)

    # Makes the directory read-only so write pidfile will fail.
    os.chmod(pid_file_path_dir, 0o555)

    fake_cclient = FakeClearcutClient()
    with self.assertRaises(PermissionError):
      dm = daemon_manager.DaemonManager(TEST_BINARY_FILE, cclient=fake_cclient)
      dm.start()
    self._assert_error_event_logged(
        fake_cclient, edit_event_pb2.EditEvent.FAILED_TO_START_EDIT_MONITOR
    )

  def test_start_failed_to_start_daemon_process(self):
    fake_cclient = FakeClearcutClient()
    with self.assertRaises(TypeError):
      dm = daemon_manager.DaemonManager(
          TEST_BINARY_FILE,
          daemon_target='wrong_target',
          daemon_args=(1),
          cclient=fake_cclient,
      )
      dm.start()
    self._assert_error_event_logged(
        fake_cclient, edit_event_pb2.EditEvent.FAILED_TO_START_EDIT_MONITOR
    )

  @mock.patch('os.execv')
  def test_monitor_reboot_with_high_memory_usage(self, mock_execv):
    fake_cclient = FakeClearcutClient()
    binary_file = tempfile.NamedTemporaryFile(
        dir=self.working_dir.name, delete=False
    )

    dm = daemon_manager.DaemonManager(
        binary_file.name,
        daemon_target=memory_consume_daemon_target,
        daemon_args=(2,),
        cclient=fake_cclient,
    )
    # set the fake total_memory_size
    dm.total_memory_size = 100 * 1024 * 1024
    dm.start()
    dm.monitor_daemon(interval=1)

    self.assertTrue(dm.max_memory_usage >= 0.02)
    self.assert_no_subprocess_running()
    self._assert_error_event_logged(
        fake_cclient,
        edit_event_pb2.EditEvent.KILLED_DUE_TO_EXCEEDED_MEMORY_USAGE,
    )
    mock_execv.assert_called_once()

  def test_monitor_daemon_subprocess_killed_high_cpu_usage(self):
    fake_cclient = FakeClearcutClient()
    dm = daemon_manager.DaemonManager(
        TEST_BINARY_FILE,
        daemon_target=cpu_consume_daemon_target,
        daemon_args=(20,),
        cclient=fake_cclient,
    )
    dm.start()
    dm.monitor_daemon(interval=1, cpu_threshold=20)

    self.assertTrue(dm.max_cpu_usage >= 20)
    self.assert_no_subprocess_running()
    self._assert_error_event_logged(
        fake_cclient,
        edit_event_pb2.EditEvent.KILLED_DUE_TO_EXCEEDED_CPU_USAGE,
    )

  @mock.patch('subprocess.check_output')
  def test_monitor_daemon_failed_does_not_matter(self, mock_output):
    mock_output.side_effect = OSError('Unknown OSError')
    self.assert_run_simple_daemon_success()

  @mock.patch('os.execv')
  def test_monitor_daemon_reboot_triggered(self, mock_execv):
    binary_file = tempfile.NamedTemporaryFile(
        dir=self.working_dir.name, delete=False
    )

    dm = daemon_manager.DaemonManager(
        binary_file.name,
        daemon_target=long_running_daemon,
    )
    dm.start()
    dm.monitor_daemon(reboot_timeout=0.5)
    mock_execv.assert_called_once()

  def test_stop_success(self):
    dm = daemon_manager.DaemonManager(
        TEST_BINARY_FILE, daemon_target=long_running_daemon
    )
    dm.start()
    dm.stop()

    self.assert_no_subprocess_running()
    self.assertFalse(dm.pid_file_path.exists())

  @mock.patch('os.kill')
  def test_stop_failed_to_kill_daemon_process(self, mock_kill):
    mock_kill.side_effect = OSError('Unknown OSError')
    fake_cclient = FakeClearcutClient()
    dm = daemon_manager.DaemonManager(
        TEST_BINARY_FILE,
        daemon_target=long_running_daemon,
        cclient=fake_cclient,
    )

    with self.assertRaises(SystemExit):
      dm.start()
      dm.stop()
      self.assertTrue(dm.daemon_process.is_alive())
      self.assertTrue(dm.pid_file_path.exists())
    self._assert_error_event_logged(
        fake_cclient, edit_event_pb2.EditEvent.FAILED_TO_STOP_EDIT_MONITOR
    )

  @mock.patch('os.remove')
  def test_stop_failed_to_remove_pidfile(self, mock_remove):
    mock_remove.side_effect = OSError('Unknown OSError')

    fake_cclient = FakeClearcutClient()
    dm = daemon_manager.DaemonManager(
        TEST_BINARY_FILE,
        daemon_target=long_running_daemon,
        cclient=fake_cclient,
    )

    with self.assertRaises(SystemExit):
      dm.start()
      dm.stop()
      self.assert_no_subprocess_running()
      self.assertTrue(dm.pid_file_path.exists())

    self._assert_error_event_logged(
        fake_cclient, edit_event_pb2.EditEvent.FAILED_TO_STOP_EDIT_MONITOR
    )

  @mock.patch('os.execv')
  def test_reboot_success(self, mock_execv):
    binary_file = tempfile.NamedTemporaryFile(
        dir=self.working_dir.name, delete=False
    )

    dm = daemon_manager.DaemonManager(
        binary_file.name, daemon_target=long_running_daemon
    )
    dm.start()
    dm.reboot()

    # Verifies the old process is stopped
    self.assert_no_subprocess_running()
    self.assertFalse(dm.pid_file_path.exists())

    mock_execv.assert_called_once()

  @mock.patch('os.execv')
  def test_reboot_binary_no_longer_exists(self, mock_execv):
    dm = daemon_manager.DaemonManager(
        TEST_BINARY_FILE, daemon_target=long_running_daemon
    )
    dm.start()

    with self.assertRaises(SystemExit):
      dm.reboot()
      mock_execv.assert_not_called()
      self.assertEqual(cm.exception.code, 0)

  @mock.patch('os.execv')
  def test_reboot_failed(self, mock_execv):
    mock_execv.side_effect = OSError('Unknown OSError')
    fake_cclient = FakeClearcutClient()
    binary_file = tempfile.NamedTemporaryFile(
        dir=self.working_dir.name, delete=False
    )

    dm = daemon_manager.DaemonManager(
        binary_file.name,
        daemon_target=long_running_daemon,
        cclient=fake_cclient,
    )
    dm.start()

    with self.assertRaises(SystemExit):
      dm.reboot()
      self.assertEqual(cm.exception.code, 1)
    self._assert_error_event_logged(
        fake_cclient, edit_event_pb2.EditEvent.FAILED_TO_REBOOT_EDIT_MONITOR
    )

  @mock.patch('subprocess.check_output')
  def test_cleanup_success(self, mock_check_output):
    p = self._create_fake_deamon_process()
    fake_cclient = FakeClearcutClient()
    mock_check_output.return_value = f'user {p.pid} 1 1 1 1 1 edit_monitor arg'

    dm = daemon_manager.DaemonManager(
        TEST_BINARY_FILE,
        daemon_target=long_running_daemon,
        cclient=fake_cclient,
    )
    dm.cleanup()

    self.assertFalse(p.is_alive())
    self.assertTrue(
        pathlib.Path(self.working_dir.name)
        .joinpath(daemon_manager.BLOCK_SIGN_FILE)
        .exists()
    )

  def assert_run_simple_daemon_success(self):
    damone_output_file = tempfile.NamedTemporaryFile(
        dir=self.working_dir.name, delete=False
    )
    dm = daemon_manager.DaemonManager(
        TEST_BINARY_FILE,
        daemon_target=simple_daemon,
        daemon_args=(damone_output_file.name,),
    )
    dm.start()
    dm.monitor_daemon(interval=1)

    # Verifies the expected pid file is created.
    expected_pid_file_path = pathlib.Path(self.working_dir.name).joinpath(
        'edit_monitor', TEST_PID_FILE_PATH
    )
    self.assertTrue(expected_pid_file_path.exists())

    # Verify the daemon process is executed successfully.
    with open(damone_output_file.name, 'r') as f:
      contents = f.read()
      self.assertEqual(contents, 'running daemon target')

  def assert_no_subprocess_running(self):
    child_pids = self._get_child_processes(os.getpid())
    for child_pid in child_pids:
      self.assertFalse(
          self._is_process_alive(child_pid), f'process {child_pid} still alive'
      )

  def _get_child_processes(self, parent_pid: int) -> list[int]:
    try:
      output = subprocess.check_output(
          ['ps', '-o', 'pid,ppid', '--no-headers'], text=True
      )

      child_processes = []
      for line in output.splitlines():
        pid, ppid = line.split()
        if int(ppid) == parent_pid:
          child_processes.append(int(pid))
      return child_processes
    except subprocess.CalledProcessError as e:
      self.fail(f'failed to get child process, error: {e}')

  def _is_process_alive(self, pid: int) -> bool:
    try:
      output = subprocess.check_output(
          ['ps', '-p', str(pid), '-o', 'state='], text=True
      ).strip()
      state = output.split()[0]
      return state != 'Z'  # Check if the state is not 'Z' (zombie)
    except subprocess.CalledProcessError:
      return False

  def _cleanup_child_processes(self):
    child_pids = self._get_child_processes(os.getpid())
    for child_pid in child_pids:
      try:
        os.kill(child_pid, signal.SIGKILL)
      except ProcessLookupError:
        # process already terminated
        pass

  def _create_fake_deamon_process(
      self, name: str = TEST_PID_FILE_PATH
  ) -> multiprocessing.Process:
    # Create a long running subprocess
    p = multiprocessing.Process(target=long_running_daemon)
    p.start()

    # Create the pidfile with the subprocess pid
    pid_file_path_dir = pathlib.Path(self.working_dir.name).joinpath(
        'edit_monitor'
    )
    pid_file_path_dir.mkdir(parents=True, exist_ok=True)
    with open(pid_file_path_dir.joinpath(name), 'w') as f:
      f.write(str(p.pid))
    return p

  def _assert_error_event_logged(self, fake_cclient, error_type):
    error_events = fake_cclient.get_sent_events()
    self.assertEqual(len(error_events), 1)
    self.assertEqual(
        edit_event_pb2.EditEvent.FromString(
            error_events[0].source_extension
        ).edit_monitor_error_event.error_type,
        error_type,
    )


  def test_aggregate_and_send_metrics(self):
    fake_cclient = FakeClearcutClient()
    dm = daemon_manager.DaemonManager(
        TEST_BINARY_FILE,
        daemon_target=simple_daemon,
        cclient=fake_cclient,
    )
    dm.total_memory_size = 1000
    cpu_samples = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]
    memory_samples = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0]
    dm._aggregate_and_send_metrics(cpu_samples, memory_samples, 60)
    sent_events = fake_cclient.get_sent_events()
    self.assertEqual(len(sent_events), 1)
    metrics_event = edit_event_pb2.EditEvent.FromString(
        sent_events[0].source_extension
    ).metrics_event
    self.assertAlmostEqual(metrics_event.cpu_usage_p50, 5.5)
    self.assertAlmostEqual(metrics_event.cpu_usage_p95, 9.55)
    self.assertAlmostEqual(metrics_event.cpu_usage_max, 10.0)
    self.assertAlmostEqual(metrics_event.memory_usage_p50, 550.0)
    self.assertAlmostEqual(metrics_event.memory_usage_p95, 955.0)
    self.assertAlmostEqual(metrics_event.memory_usage_max, 1000.0)
    self.assertEqual(metrics_event.time_window_seconds, 60)

  def test_get_percentile_linear_interpolation(self):
    dm = daemon_manager.DaemonManager(TEST_BINARY_FILE)
    samples = [1.0, 2.0, 3.0, 4.0, 5.0]
    # P50 (median) of [1, 2, 3, 4, 5] should be 3.0
    self.assertAlmostEqual(dm._get_percentile(samples, 0.5), 3.0)
    # P95: index = 4 * 0.95 = 3.8. lower = 3, upper = 4.
    # value = samples[3] * 0.2 + samples[4] * 0.8 = 4.0 * 0.2 + 5.0 * 0.8 = 0.8 + 4.0 = 4.8
    self.assertAlmostEqual(dm._get_percentile(samples, 0.95), 4.8)

  def test_get_percentile_empty_samples(self):
    dm = daemon_manager.DaemonManager(TEST_BINARY_FILE)
    self.assertEqual(dm._get_percentile([], 0.95), 0.0)

  def test_get_percentile_single_sample(self):
    dm = daemon_manager.DaemonManager(TEST_BINARY_FILE)
    self.assertEqual(dm._get_percentile([10.0], 0.5), 10.0)
    self.assertEqual(dm._get_percentile([10.0], 0.95), 10.0)

  def test_aggregate_and_send_metrics_empty(self):
    fake_cclient = FakeClearcutClient()
    dm = daemon_manager.DaemonManager(
        TEST_BINARY_FILE,
        daemon_target=simple_daemon,
        cclient=fake_cclient,
    )
    dm._aggregate_and_send_metrics([], [], 60)
    self.assertEqual(len(fake_cclient.get_sent_events()), 0)

class FakeClearcutClient:

  def __init__(self):
    self.pending_log_events = []
    self.sent_log_event = []

  def log(self, log_event):
    self.pending_log_events.append(log_event)

  def flush_events(self):
    self.sent_log_event.extend(self.pending_log_events)
    self.pending_log_events.clear()

  def get_sent_events(self):
    return self.sent_log_event + self.pending_log_events


if __name__ == '__main__':
  unittest.main()
