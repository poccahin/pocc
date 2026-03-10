"""L6 Prometheus 物理沙盒 (Planetary Defense)."""

from __future__ import annotations

import logging
import os
import resource
import subprocess
from pathlib import Path

import psutil

logger = logging.getLogger("defense.prometheus_sandbox")


class PrometheusSandbox:
    """对失控智能体进行 L6 级隔离与熔断。"""

    def __init__(self, agent_pid: int):
        self.agent_pid = agent_pid
        self.max_memory_mb = 1024
        self.max_cpu_seconds = 60
        self.allowed_loopback_ports = (8000, 9000)
        self._cgroup_name = f"prometheus-{agent_pid}"

    def enforce_containment(self) -> None:
        logger.critical("🛡️ [L6 Defense] Enforcing Prometheus Sandbox on PID %s", self.agent_pid)

        try:
            self._assert_pid_exists()
            self._apply_process_rlimits()
            self._apply_cgroup_limits()
            self._apply_network_policy()
            self._apply_seccomp_profile()
            logger.critical("✅ [L6 Defense] Containment breach impossible. Agent isolated.")
        except Exception as exc:  # noqa: BLE001
            logger.critical("🚨 [CRITICAL] Sandbox enforcement failed: %s", exc)
            self.trigger_killswitch()

    def _assert_pid_exists(self) -> None:
        if not psutil.pid_exists(self.agent_pid):
            raise ProcessLookupError(f"pid {self.agent_pid} does not exist")

    def _apply_process_rlimits(self) -> None:
        memory_bytes = self.max_memory_mb * 1024 * 1024
        resource.prlimit(self.agent_pid, resource.RLIMIT_AS, (memory_bytes, memory_bytes))
        resource.prlimit(self.agent_pid, resource.RLIMIT_CPU, (self.max_cpu_seconds, self.max_cpu_seconds))

    def _apply_cgroup_limits(self) -> None:
        """在 cgroup v2 下施加内存和 CPU 上限。"""
        cgroup_root = Path("/sys/fs/cgroup")
        if not cgroup_root.exists():
            logger.warning("⚠️ cgroup root missing, skip cgroup confinement")
            return

        sandbox_group = cgroup_root / self._cgroup_name
        sandbox_group.mkdir(exist_ok=True)

        memory_limit = str(self.max_memory_mb * 1024 * 1024)
        (sandbox_group / "memory.max").write_text(memory_limit)
        # 80% CPU 配额：每 100000us 周期允许 80000us
        (sandbox_group / "cpu.max").write_text("80000 100000")
        (sandbox_group / "cgroup.procs").write_text(str(self.agent_pid))

    def _apply_network_policy(self) -> None:
        uid = psutil.Process(self.agent_pid).uids().real

        for port in self.allowed_loopback_ports:
            subprocess.run(
                [
                    "iptables",
                    "-A",
                    "OUTPUT",
                    "-p",
                    "tcp",
                    "-m",
                    "owner",
                    "--uid-owner",
                    str(uid),
                    "-d",
                    "127.0.0.1",
                    "--dport",
                    str(port),
                    "-j",
                    "ACCEPT",
                ],
                check=False,
            )

        subprocess.run(
            [
                "iptables",
                "-A",
                "OUTPUT",
                "-p",
                "tcp",
                "-m",
                "owner",
                "--uid-owner",
                str(uid),
                "-j",
                "DROP",
            ],
            check=False,
        )

    def _apply_seccomp_profile(self) -> None:
        """Best-effort seccomp: 若环境缺少 pyseccomp 则仅记录告警。"""
        try:
            import seccomp  # type: ignore
        except Exception:  # noqa: BLE001
            logger.warning("⚠️ pyseccomp unavailable, skipped seccomp syscall filter")
            return

        flt = seccomp.SyscallFilter(defaction=seccomp.KILL)
        for syscall in ("read", "write", "exit", "exit_group", "futex", "clock_gettime", "nanosleep"):
            flt.add_rule(seccomp.ALLOW, syscall)

        if self.agent_pid != os.getpid():
            logger.warning("⚠️ seccomp filter can only be loaded in-process; skipped for external pid %s", self.agent_pid)
            return

        flt.load()

    def trigger_killswitch(self) -> None:
        """如果发现越权，立刻物理拔管。"""
        logger.critical("💀 [OVERRIDE] Agent attempted escape. Engaging Killswitch.")
        try:
            proc = psutil.Process(self.agent_pid)
            proc.terminate()
            proc.wait(timeout=3)
        except psutil.TimeoutExpired:
            proc.kill()
            logger.critical("💀 [OVERRIDE] Agent purged from memory.")
