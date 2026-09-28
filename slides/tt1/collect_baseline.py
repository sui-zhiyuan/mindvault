#!/usr/bin/env python3
"""collect_baseline.py — 收集 CubeSandbox 双平台对比测试的「公平性基线」证据。

用途
----
两台机器（x86_64 与鲲鹏 950/aarch64）在做性能对比之前，必须先把会影响
性能、又容易被忽略的环境事实记录下来，否则两组数据无法归因。

本脚本把《CubeSandbox 源码构建与双平台对比测试手册》第 2 篇要求的各项
证据一次性采集为一份 JSON，便于归档与并排比对。

设计约束
--------
* **只读**：不修改任何系统配置，不写系统路径（只写 --out 指定的文件）。
* **零依赖**：仅用 Python 3 标准库，不需要 pip 安装任何东西。
* **缺命令不致命**：某个工具（如 numactl / dmidecode）不存在时记录为 null
  并在 ``_errors`` 中说明，不影响其余字段采集。
* **双架构**：x86_64 与 aarch64 均可运行；架构相关字段按能力探测，不做假设。

用法
----
    python3 collect_baseline.py                      # 输出 baseline-<arch>.json
    python3 collect_baseline.py --out b.json --pretty
    python3 collect_baseline.py --quiet              # 只输出 JSON

建议在**部署前**与**部署后**各跑一次，分别归档。
"""

from __future__ import annotations

import argparse
import glob
import json
import os
import platform
import shutil
import subprocess
import sys
import time

# --------------------------------------------------------------------------
# 基础工具
# --------------------------------------------------------------------------

#: 采集过程中的错误，最后并入输出的 _errors 字段
ERRORS: list[str] = []


def run(cmd: list[str], timeout: int = 20) -> str | None:
    """执行命令并返回 stdout（已去首尾空白）。

    命令不存在、超时、非零退出时返回 None，并记入 ERRORS。
    绝不抛异常，保证采集流程不中断。
    """
    exe = cmd[0]
    if shutil.which(exe) is None:
        ERRORS.append(f"command not found: {exe}")
        return None
    try:
        proc = subprocess.run(
            cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            timeout=timeout,
            check=False,
        )
    except subprocess.TimeoutExpired:
        ERRORS.append(f"timeout ({timeout}s): {' '.join(cmd)}")
        return None
    except OSError as exc:  # 权限、可执行位等
        ERRORS.append(f"oserror running {' '.join(cmd)}: {exc}")
        return None
    if proc.returncode != 0:
        ERRORS.append(f"exit {proc.returncode}: {' '.join(cmd)}")
        return None
    return proc.stdout.decode("utf-8", "replace").strip()


def read_text(path: str) -> str | None:
    """读取一个小文本文件；不存在或不可读时返回 None。"""
    try:
        with open(path, "r", encoding="utf-8", errors="replace") as handle:
            return handle.read().strip()
    except OSError:
        return None


def read_int(path: str) -> int | None:
    """读取只含一个整数的 sysfs/procfs 文件。"""
    raw = read_text(path)
    if raw is None:
        return None
    try:
        return int(raw.split()[0])
    except (ValueError, IndexError):
        return None


def glob_int(path: str) -> int | None:
    """读取形如 ``/sys/.../cpu0/cache/index3/size``（可能是 "32768K"）的值。"""
    raw = read_text(path)
    if raw is None:
        return None
    raw = raw.strip().upper()
    try:
        if raw.endswith("K"):
            return int(raw[:-1]) * 1024
        if raw.endswith("M"):
            return int(raw[:-1]) * 1024 * 1024
        return int(raw)
    except ValueError:
        return None


def block(title: str, fn):
    """执行一个采集块，捕获任何异常并记入 ERRORS，保证其它块继续。"""
    try:
        return fn()
    except Exception as exc:  # noqa: BLE001 - 采集器必须永不崩溃
        ERRORS.append(f"{title}: {type(exc).__name__}: {exc}")
        return None


# --------------------------------------------------------------------------
# 各采集块
# --------------------------------------------------------------------------


def collect_identity() -> dict:
    """主机与采集时间。"""
    return {
        "collected_at_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "hostname": platform.node(),
        "machine": platform.machine(),          # x86_64 / aarch64
        "system": platform.system(),
        "python": platform.python_version(),
    }


def collect_os() -> dict:
    """发行版、内核、启动参数、运行时长。"""
    os_release: dict[str, str] = {}
    raw = read_text("/etc/os-release") or ""
    for line in raw.splitlines():
        if "=" in line:
            key, _, value = line.partition("=")
            os_release[key.strip()] = value.strip().strip('"')

    return {
        "os_release": os_release or None,
        "kernel_release": run(["uname", "-r"]),
        "kernel_version": run(["uname", "-v"]),
        "kernel_cmdline": read_text("/proc/cmdline"),
        "uptime_seconds": (
            float(read_text("/proc/uptime").split()[0])
            if read_text("/proc/uptime")
            else None
        ),
    }


def collect_cpu() -> dict:
    """CPU 型号、SKU、核数、SMT、频率范围、缓存。"""
    lscpu = run(["lscpu"])

    # 从 lscpu 中提取关键字段，便于程序化比对
    lscpu_fields: dict[str, str] = {}
    if lscpu:
        for line in lscpu.splitlines():
            if ":" in line:
                key, _, value = line.partition(":")
                lscpu_fields[key.strip()] = value.strip()

    # /proc/cpuinfo 的型号串（比 lscpu 更接近 BIOS 原始字符串）
    model_name = None
    hardware = None
    for line in (read_text("/proc/cpuinfo") or "").splitlines():
        if line.lower().startswith("model name") and model_name is None:
            model_name = line.split(":", 1)[1].strip()
        elif line.lower().startswith("hardware") and hardware is None:
            hardware = line.split(":", 1)[1].strip()

    # SMT 实际是否开启 —— 公平性关键项（x86 默认开，鲲鹏可能默认关）
    smt_active = read_int("/sys/devices/system/cpu/smt/active")
    smt_control = read_text("/sys/devices/system/cpu/smt/control")

    return {
        # 完整原文，供人工核对
        "lscpu_raw": lscpu,
        "lscpu_fields": lscpu_fields or None,
        "proc_cpuinfo_model_name": model_name,
        "proc_cpuinfo_hardware": hardware,
        "logical_cpus_os_cpu_count": os.cpu_count(),
        "smt": {
            "active": smt_active,            # 1=开 0=关
            "control": smt_control,          # on / off / forceoff / notsupported
            "thread_siblings_list": read_text(
                "/sys/devices/system/cpu/cpu0/topology/thread_siblings_list"
            ),
        },
        "topology": {
            "core_id_cpu0": read_int("/sys/devices/system/cpu/cpu0/topology/core_id"),
            "physical_package_id_cpu0": read_int(
                "/sys/devices/system/cpu/cpu0/topology/physical_package_id"
            ),
        },
        "cache": {
            # 鲲鹏/AMD 的 L3 归属（CCX / SCCL）会影响对比解释
            "l1d_size_bytes": glob_int("/sys/devices/system/cpu/cpu0/cache/index0/size"),
            "l1i_size_bytes": glob_int("/sys/devices/system/cpu/cpu0/cache/index1/size"),
            "l2_size_bytes": glob_int("/sys/devices/system/cpu/cpu0/cache/index2/size"),
            "l3_size_bytes": glob_int("/sys/devices/system/cpu/cpu0/cache/index3/size"),
            "l3_shared_cpu_list": read_text(
                "/sys/devices/system/cpu/cpu0/cache/index3/shared_cpu_list"
            ),
        },
        # ISA 能力：涉及向量化的负载需要按 ISA 分开报告
        "isa_flags": _isa_flags(),
    }


def _isa_flags() -> dict:
    """挑出对性能对比有意义的 ISA 扩展。"""
    features = set()
    for line in (read_text("/proc/cpuinfo") or "").splitlines():
        low = line.lower()
        if low.startswith("flags") or low.startswith("features"):
            _, _, value = line.partition(":")
            features.update(value.split())
    interesting = [
        "avx2", "avx512f", "avx512bw", "avx512vl",
        "sve", "sve2", "asimd", "neon", "fp", "crc32",
    ]
    return {
        "present": sorted(f for f in interesting if f in features),
        "avx512_any": any(f.startswith("avx512") for f in features),
        "sve_any": any(f.startswith("sve") for f in features),
    }


def collect_memory() -> dict:
    """内存总量与 DIMM 细节（通道/速率直接影响带宽对比）。"""
    meminfo: dict[str, int] = {}
    for line in (read_text("/proc/meminfo") or "").splitlines():
        key, _, value = line.partition(":")
        parts = value.split()
        if parts and parts[0].isdigit():
            meminfo[key.strip()] = int(parts[0])

    dmidecode_memory = run(["dmidecode", "-t", "memory"], timeout=30)

    return {
        "meminfo_kb": meminfo or None,
        "memtotal_kb": meminfo.get("MemTotal"),
        "dmidecode_memory_raw": dmidecode_memory,
        # 关键速率字段单独抽出，方便两台机器并排看
        "configured_memory_speed": _extract_dmi_field(
            dmidecode_memory, "Configured Memory Speed"
        ),
        "speed": _extract_dmi_field(dmidecode_memory, "Speed"),
        "dimm_count": (
            dmidecode_memory.count("Memory Device")
            if dmidecode_memory
            else None
        ),
        "numa": collect_numa(),
    }


def _extract_dmi_field(raw: str | None, field: str) -> list[str]:
    """从 dmidecode 输出里抽取某个字段的所有取值（去重）。"""
    if not raw:
        return []
    seen: list[str] = []
    for line in raw.splitlines():
        stripped = line.strip()
        if stripped.startswith(field + ":"):
            value = stripped.split(":", 1)[1].strip()
            if value and value not in seen:
                seen.append(value)
    return seen


def collect_numa() -> dict:
    """NUMA 拓扑 —— 公平性关键项（AMD NPS / 鲲鹏 SCCL 语义不同）。"""
    node_ids = sorted(
        int(os.path.basename(p).replace("node", ""))
        for p in glob.glob("/sys/devices/system/node/node[0-9]*")
    )
    nodes = {}
    for node in node_ids:
        nodes[f"node{node}"] = {
            "cpulist": read_text(f"/sys/devices/system/node/node{node}/cpulist"),
            "meminfo_kb": None,
        }
    return {
        "node_count": len(node_ids) or None,
        "numactl_H": run(["numactl", "-H"]),
        "numa_balancing": read_text("/proc/sys/kernel/numa_balancing"),
        "nodes": nodes or None,
    }


def collect_pages() -> dict:
    """页面大小 / 大页 / THP —— 架构固有差异，必须如实记录。"""
    page_size = None
    try:
        page_size = os.sysconf("SC_PAGE_SIZE")
    except (ValueError, OSError):
        page_size = None

    meminfo = read_text("/proc/meminfo") or ""
    huge = {}
    for line in meminfo.splitlines():
        if "Huge" in line:
            key, _, value = line.partition(":")
            huge[key.strip()] = value.strip()

    return {
        "page_size_bytes": page_size,
        "page_size_via_getconf": run(["getconf", "PAGESIZE"]),
        "transparent_hugepage": {
            "enabled": read_text("/sys/kernel/mm/transparent_hugepage/enabled"),
            "defrag": read_text("/sys/kernel/mm/transparent_hugepage/defrag"),
            "shmem_enabled": read_text(
                "/sys/kernel/mm/transparent_hugepage/shmem_enabled"
            ),
        },
        "hugepages_meminfo": huge or None,
        "nr_hugepages": read_int("/proc/sys/vm/nr_hugepages"),
        "nr_overcommit_hugepages": read_int("/proc/sys/vm/nr_overcommit_hugepages"),
        "hugepages_cmdline": _hugepage_cmdline(),
    }


def _hugepage_cmdline() -> list[str]:
    """把 cmdline 里的大页相关参数单独列出。"""
    cmdline = read_text("/proc/cmdline") or ""
    return [tok for tok in cmdline.split() if "hugepage" in tok.lower()]


def collect_power() -> dict:
    """频率调节与空闲状态 —— 决定频率是否稳定（数据可比性的核心）。"""
    governors = {}
    for gov_path in sorted(glob.glob("/sys/devices/system/cpu/cpu*/cpufreq/scaling_governor")):
        value = read_text(gov_path)
        if value:
            governors[value] = governors.get(value, 0) + 1

    drivers = {}
    for drv_path in sorted(
        glob.glob("/sys/devices/system/cpu/cpu*/cpufreq/scaling_driver")
    ):
        value = read_text(drv_path)
        if value:
            drivers[value] = drivers.get(value, 0) + 1

    return {
        "scaling_governor_histogram": governors or None,
        "scaling_driver_histogram": drivers or None,
        "cpupower_frequency_info": run(["cpupower", "frequency-info"]),
        "cpupower_idle_info": run(["cpupower", "idle-info"]),
        "cpufreq_boost": read_text("/sys/devices/system/cpu/cpufreq/boost"),
        "intel_pstate_no_turbo": read_text(
            "/sys/devices/system/cpu/intel_pstate/no_turbo"
        ),
        "amd_pstate_status": read_text("/sys/devices/system/cpu/amd_pstate/status"),
    }


def collect_security() -> dict:
    """漏洞缓解状态 —— 会影响性能，两边需可比。"""
    mitigations = {}
    for path in sorted(glob.glob("/sys/devices/system/cpu/vulnerabilities/*")):
        mitigations[os.path.basename(path)] = read_text(path)
    return {
        "vulnerabilities": mitigations or None,
        "selinux": run(["getenforce"]),
        "cmdline_mitigations": [
            tok for tok in (read_text("/proc/cmdline") or "").split()
            if tok.startswith("mitigations") or "pti" in tok.lower()
        ],
    }


def collect_virtualization() -> dict:
    """KVM / IOMMU 能力 —— 决定 MicroVM 能否以原生方式运行。"""
    return {
        "dev_kvm_exists": os.path.exists("/dev/kvm"),
        "dev_kvm_stat": _stat_repr("/dev/kvm"),
        "kvm_modules": run(["lsmod"]) and _filter_lines(run(["lsmod"]), ("kvm",)),
        "iommu_groups": len(glob.glob("/sys/kernel/iommu_groups/*")) or None,
        "bpf_filesystems": (
            "bpf" in (read_text("/proc/filesystems") or "")
        ),
        "sys_fs_bpf_mounted": os.path.ismount("/sys/fs/bpf"),
        "cgroup_version": _cgroup_version(),
        "cgroup_controllers": read_text("/sys/fs/cgroup/cgroup.controllers"),
        "cgroup_subtree_control": read_text("/sys/fs/cgroup/cgroup.subtree_control"),
    }


def _filter_lines(raw: str | None, needles: tuple[str, ...]) -> list[str] | None:
    if not raw:
        return None
    return [ln for ln in raw.splitlines() if any(n in ln for n in needles)]


def _cgroup_version() -> str | None:
    """返回 v1 / v2 / hybrid。"""
    if os.path.exists("/sys/fs/cgroup/cgroup.controllers"):
        return "v2"
    if os.path.exists("/sys/fs/cgroup/cpu"):
        return "v1"
    return None


def _stat_repr(path: str) -> dict | None:
    try:
        st = os.stat(path)
    except OSError:
        return None
    return {
        "mode": oct(st.st_mode),
        "uid": st.st_uid,
        "gid": st.st_gid,
        "rdev": st.st_rdev,
    }


def collect_firmware() -> dict:
    """BIOS/BMC/整机信息。

    Power Profile、Hardware Prefetcher 等鲲鹏 BIOS 项**无法从 OS 反推**，
    必须靠 BIOS/Redfish 导出；本字段只提供可以在 OS 侧拿到的部分作为线索。
    """
    return {
        "dmidecode_bios": run(["dmidecode", "-t", "bios"], timeout=20),
        "dmidecode_system": run(["dmidecode", "-t", "system"], timeout=20),
        "dmidecode_baseboard": run(["dmidecode", "-t", "baseboard"], timeout=20),
        "dmidecode_processor": run(["dmidecode", "-t", "processor"], timeout=30),
        "note": (
            "Power Profile / Hardware Prefetcher / UFS / LPI 等 BIOS 项无法从 OS 读取；"
            "请另附 Redfish BIOS Attributes JSON 或 BIOS 截图。"
        ),
    }


def collect_storage() -> dict:
    """磁盘与文件系统 —— /data/cubelet 必须是 XFS（reflink）。"""
    targets = ["/", "/data", "/usr/local/services/cubetoolbox", "/tmp", "/home"]
    fs_info = {}
    for target in targets:
        fs_info[target] = {
            "exists": os.path.exists(target),
            "df_T": run(["df", "-T", target]) if os.path.exists(target) else None,
            "mountpoint": os.path.ismount(target) if os.path.exists(target) else None,
        }

    return {
        "lsblk_json": run(["lsblk", "-J", "-o",
                           "NAME,SIZE,TYPE,FSTYPE,MOUNTPOINT,MODEL,ROTA"], timeout=20),
        "df_hT": run(["df", "-hT"], timeout=20),
        "targets": fs_info,
        "xfs_info_data": run(["xfs_info", "/data"]) if os.path.exists("/data") else None,
        "xfs_info_cubelet": (
            run(["xfs_info", "/data/cubelet"])
            if os.path.exists("/data/cubelet")
            else None
        ),
    }


def collect_docker() -> dict:
    """Docker 版本与镜像 digest —— 容器侧一致性证据。"""
    return {
        "docker_version": run(["docker", "--version"]),
        "docker_info": run(["docker", "info"], timeout=30),
        "docker_compose_version": run(["docker-compose", "--version"]),
        "docker_compose_plugin_version": run(["docker", "compose", "version"]),
        "images_with_digests": run(["docker", "images", "--digests", "--no-trunc"], timeout=30),
        "running_containers": run(
            ["docker", "ps", "--format", "{{.Names}}\t{{.Image}}\t{{.Status}}"], timeout=30
        ),
    }


def collect_cubesandbox() -> dict:
    """若已安装 CubeSandbox，采集其版本指纹（部署前会是 null）。"""
    prefix = "/usr/local/services/cubetoolbox"
    manifest_path = os.path.join(prefix, "release-manifest.json")
    manifest = read_text(manifest_path)
    parsed = None
    if manifest:
        try:
            parsed = json.loads(manifest)
        except json.JSONDecodeError:
            ERRORS.append("release-manifest.json is not valid JSON")

    return {
        "install_prefix": prefix,
        "installed": os.path.isdir(prefix),
        "version_txt": read_text(os.path.join(prefix, "VERSION.txt")),
        "release_manifest": parsed,
        "one_click_env_keys": _env_keys(os.path.join(prefix, ".one-click.env")),
        "systemd_units": run(
            ["systemctl", "list-units", "cube-sandbox-*", "--no-pager", "--all"],
            timeout=20,
        ),
    }


def _env_keys(path: str) -> list[str] | None:
    """只取 .one-click.env 的键名，**不取键值**（避免把口令写进证据文件）。"""
    raw = read_text(path)
    if raw is None:
        return None
    keys = []
    for line in raw.splitlines():
        line = line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        keys.append(line.split("=", 1)[0])
    return sorted(keys)


# --------------------------------------------------------------------------
# 主流程
# --------------------------------------------------------------------------

COLLECTORS = [
    ("identity", collect_identity),
    ("os", collect_os),
    ("cpu", collect_cpu),
    ("memory", collect_memory),
    ("pages", collect_pages),
    ("power", collect_power),
    ("security", collect_security),
    ("virtualization", collect_virtualization),
    ("firmware", collect_firmware),
    ("storage", collect_storage),
    ("docker", collect_docker),
    ("cubesandbox", collect_cubesandbox),
]


def main() -> int:
    parser = argparse.ArgumentParser(
        description="收集 CubeSandbox 双平台对比测试的公平性基线证据（只读）。",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--out",
        default=None,
        help="输出 JSON 路径（默认 baseline-<arch>.json）",
    )
    parser.add_argument("--pretty", action="store_true", help="美化 JSON 输出")
    parser.add_argument("--quiet", action="store_true", help="不打印进度信息")
    args = parser.parse_args()

    out_path = args.out or f"baseline-{platform.machine()}.json"

    if not args.quiet:
        print(f"[collect_baseline] arch={platform.machine()} out={out_path}", file=sys.stderr)

    report: dict = {}
    for title, fn in COLLECTORS:
        if not args.quiet:
            print(f"[collect_baseline] collecting: {title}", file=sys.stderr)
        report[title] = block(title, fn)

    report["_errors"] = ERRORS

    # 公平性提示：把最容易导致不可比的项目顶到最前面
    report["_fairness_highlights"] = _highlights(report)

    payload = json.dumps(
        report, indent=2 if args.pretty else None, ensure_ascii=False, sort_keys=True
    )
    try:
        with open(out_path, "w", encoding="utf-8") as handle:
            handle.write(payload + "\n")
    except OSError as exc:
        print(f"[collect_baseline] cannot write {out_path}: {exc}", file=sys.stderr)
        return 1

    if not args.quiet:
        print(f"[collect_baseline] wrote {out_path} ({len(payload)} bytes)", file=sys.stderr)
        if ERRORS:
            print(f"[collect_baseline] {len(ERRORS)} warning(s); see _errors", file=sys.stderr)
    else:
        print(payload)
    return 0


def _highlights(report: dict) -> dict:
    """抽出对比时最先要看的几项，避免被大段原文淹没。"""
    cpu = report.get("cpu") or {}
    pages = report.get("pages") or {}
    numa = (report.get("memory") or {}).get("numa") or {}
    virt = report.get("virtualization") or {}
    power = report.get("power") or {}

    return {
        "arch": (report.get("identity") or {}).get("machine"),
        "cpu_model": cpu.get("proc_cpuinfo_model_name")
        or cpu.get("proc_cpuinfo_hardware"),
        "logical_cpus": cpu.get("logical_cpus_os_cpu_count"),
        "smt_active": (cpu.get("smt") or {}).get("active"),
        "numa_node_count": numa.get("node_count"),
        "numa_balancing": numa.get("numa_balancing"),
        "page_size_bytes": pages.get("page_size_bytes"),
        "thp_enabled": (pages.get("transparent_hugepage") or {}).get("enabled"),
        "hugepages": pages.get("hugepages_cmdline"),
        "governors": power.get("scaling_governor_histogram"),
        "kvm_present": virt.get("dev_kvm_exists"),
        "bpf_fs": virt.get("bpf_filesystems"),
        "cgroup_version": virt.get("cgroup_version"),
        "warning": (
            "对比前必须确认：SMT 状态、NUMA 节点数、页面大小、governor、"
            "大页设置在两台机器上是否一致或已如实记录。"
        ),
    }


if __name__ == "__main__":
    sys.exit(main())
